//! The host process: one per user, owning every hosted session, the session index,
//! and the control and hook sockets.

mod claude_usage;
mod close;
mod details;
mod history;
mod outside;
mod usage;

use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read};
use std::net::Shutdown;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use hq_config::SettingsWatcher;
use hq_discovery::{Platform, SearchContext};
use hq_protocol::{
    ErrorCode, Frame, GroupInfo, Request, Response, SessionInfo, answer_hello, read_frame,
    write_frame,
};
use hq_signals::adapters::{self, Conversation, LaunchContext, OwnerSettings, hook};
use hq_signals::event::{Agent, EventKind, Source};
use hq_signals::external;
use hq_signals::forward::ForwardedPayload;
use hq_signals::helpers::SESSION_ENV;
use hq_signals::history::{HistoryReader, HistoryRoots};
use hq_signals::now_ms;
use hq_signals::scanner::{PROGRAM_STATUS_SUPPORT_REPLY, Scanner, TerminalSignal};
use hq_signals::status::{SessionTracker, Status, Thresholds};
use hq_store::{Origin, SessionRow, Store, StoreError};
use portable_pty::{Child, CommandBuilder, PtySize, native_pty_system};

use crate::log::Log;
use crate::paths::HostPaths;
use crate::session::{Hosted, HostedParts, Outgoing, Viewer};

const TICK: Duration = Duration::from_millis(250);
/// Output activity is fed to the status machine at most this often.
const OUTPUT_ACTIVITY_MS: u64 = 200;
/// How long `stop` waits for agents to exit after SIGHUP before leaving.
const STOP_GRACE: Duration = Duration::from_secs(3);
/// Shortest id prefix accepted as a session reference.
const MIN_ID_PREFIX: usize = 4;

pub struct HostOptions {
    pub paths: HostPaths,
    /// Executable providing `hook-forward` and `hook-statusline` (the `hq` binary).
    pub helper_exe: PathBuf,
    /// Exit after this long with no live sessions and no connected clients.
    pub idle_exit: Duration,
    /// How often Codex usage is read while a client shows usage.
    pub usage_interval: Duration,
    /// How often the agents' history is read again.
    pub history_interval: Duration,
}

#[derive(Debug)]
pub enum HostError {
    AlreadyRunning,
    Io(&'static str, std::io::Error),
    Store(StoreError),
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::AlreadyRunning => f.write_str("the Headquarter host is already running"),
            HostError::Io(what, error) => write!(f, "{what}: {error}"),
            HostError::Store(error) => write!(f, "session index: {error}"),
        }
    }
}

impl std::error::Error for HostError {}

/// Runs the host until it is stopped or idle; the process exits from inside.
pub fn run(options: HostOptions) -> Result<(), HostError> {
    let paths = options.paths;
    std::fs::create_dir_all(&paths.dir).map_err(|error| HostError::Io("data folder", error))?;
    std::fs::set_permissions(&paths.dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| HostError::Io("data folder permissions", error))?;

    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&paths.lock)
        .map_err(|error| HostError::Io("lock file", error))?;
    match lock.try_lock() {
        Ok(()) => {}
        Err(std::fs::TryLockError::WouldBlock) => return Err(HostError::AlreadyRunning),
        Err(std::fs::TryLockError::Error(error)) => return Err(HostError::Io("lock file", error)),
    }

    // Leave the launching terminal's session so closing that terminal cannot end us.
    let _ = rustix::process::setsid();

    // Only the lock holder may remove sockets, so a stale file from a crash is safe
    // to delete here.
    let control = bind_private(&paths.control_socket)?;
    let hooks = bind_private(&paths.hook_socket)?;

    let store = Store::open(&paths.database).map_err(HostError::Store)?;
    let recovered = store
        .recover_interrupted(now_ms())
        .map_err(HostError::Store)?;

    let host = Arc::new(Host {
        log: Log::open(paths.log.clone()),
        paths,
        helper_exe: options.helper_exe,
        idle_exit: options.idle_exit,
        usage_interval: options.usage_interval,
        history_interval: options.history_interval,
        store: Mutex::new(store),
        live: Mutex::new(HashMap::new()),
        subscribers: Mutex::new(Vec::new()),
        usage: Mutex::new(usage::UsageBook::default()),
        details: Mutex::new(details::DetailsBook::default()),
        clients: AtomicUsize::new(0),
        next_viewer: AtomicU64::new(1),
        stopping: AtomicBool::new(false),
        history: Mutex::new(HistoryReader::new(HistoryRoots::standard())),
        titles: Mutex::new(HashMap::new()),
        agy_conversations: Mutex::new(Vec::new()),
        outside: Mutex::new(outside::OutsideView::default()),
        outside_wake: outside::OutsideWake::default(),
        first_import: history::FirstImport::default(),
        settings: Mutex::new(SettingsWatcher::new(
            hq_config::settings_path(),
            hq_discovery::KNOWN_AGENTS
                .iter()
                .map(|agent| agent.id)
                .collect(),
        )),
        platform: hq_discovery::platform::detect(),
        login_path: Mutex::new(None),
        _lock: lock,
    });
    host.log.line(&format!(
        "host started pid={} marked_inactive={recovered}",
        std::process::id()
    ));

    host.spawn_signal_handler();
    {
        let host = Arc::clone(&host);
        thread::spawn(move || host.serve_hooks(hooks));
    }
    {
        let host = Arc::clone(&host);
        thread::spawn(move || host.tick_loop());
    }
    {
        let host = Arc::clone(&host);
        thread::spawn(move || host.capture_login_path());
    }
    {
        let host = Arc::clone(&host);
        thread::spawn(move || host.history_loop());
    }
    {
        let host = Arc::clone(&host);
        thread::spawn(move || host.outside_loop());
    }
    {
        let host = Arc::clone(&host);
        thread::spawn(move || host.details_loop());
    }
    host.load_usage();
    {
        let host = Arc::clone(&host);
        thread::spawn(move || host.usage_loop());
    }
    for stream in control.incoming() {
        let Ok(stream) = stream else { continue };
        let host = Arc::clone(&host);
        thread::spawn(move || host.serve(stream));
    }
    Ok(())
}

fn bind_private(path: &Path) -> Result<UnixListener, HostError> {
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path).map_err(|error| HostError::Io("socket", error))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| HostError::Io("socket permissions", error))?;
    Ok(listener)
}

/// Errors are boxed responses: `Response` is large and most calls succeed.
type OpResult<T> = Result<T, Box<Response>>;

fn fail(code: ErrorCode, message: impl Into<String>) -> Box<Response> {
    Box::new(Response::error(code, message))
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn bad_request(message: String) -> Box<Response> {
    fail(ErrorCode::BadRequest, message)
}

fn storage_error(error: StoreError) -> Box<Response> {
    fail(ErrorCode::Storage, error.to_string())
}

struct Host {
    paths: HostPaths,
    helper_exe: PathBuf,
    idle_exit: Duration,
    usage_interval: Duration,
    history_interval: Duration,
    store: Mutex<Store>,
    live: Mutex<HashMap<String, Arc<Hosted>>>,
    subscribers: Mutex<Vec<Subscriber>>,
    usage: Mutex<usage::UsageBook>,
    /// Sessions' details, in memory only.
    details: Mutex<details::DetailsBook>,
    clients: AtomicUsize,
    next_viewer: AtomicU64,
    stopping: AtomicBool,
    log: Log,
    history: Mutex<HistoryReader>,
    /// Agent titles by (agent, conversation id), for display only; never stored.
    titles: Mutex<HashMap<(Agent, String), String>>,
    /// agy's top-level conversations as the last history pass read them, for linking
    /// agy processes in other terminals.
    agy_conversations: Mutex<Vec<outside::AgyConversation>>,
    outside: Mutex<outside::OutsideView>,
    outside_wake: outside::OutsideWake,
    first_import: history::FirstImport,
    settings: Mutex<SettingsWatcher>,
    platform: Platform,
    /// The owner's login-shell `PATH`, captured once in the background at startup.
    login_path: Mutex<Option<String>>,
    /// Held for the host's lifetime: the single-instance lock.
    _lock: std::fs::File,
}

/// A client receiving changes; `usage` also gets usage readings.
struct Subscriber {
    /// The connection subscribed; its subscriptions end with it.
    connection: u64,
    tx: mpsc::Sender<Outgoing>,
    usage: bool,
    /// The session whose details this client shows.
    follows: Option<String>,
    /// Whether the client's window can be seen; true until it says otherwise.
    visible: bool,
}

struct ClientGuard<'a>(&'a AtomicUsize);

impl<'a> ClientGuard<'a> {
    fn new(clients: &'a AtomicUsize) -> Self {
        clients.fetch_add(1, Ordering::SeqCst);
        ClientGuard(clients)
    }
}

impl Drop for ClientGuard<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

struct Spawned {
    hosted: Arc<Hosted>,
    reader: Box<dyn Read + Send>,
    child: Box<dyn Child + Send + Sync>,
}

struct SpawnPlan<'a> {
    id: &'a str,
    agent: Agent,
    folder: &'a str,
    program: PathBuf,
    conversation: Conversation<'a>,
    extra_args: Vec<String>,
    term: Option<String>,
}

impl Host {
    // ---- connections -----------------------------------------------------------

    fn serve(self: Arc<Self>, stream: UnixStream) {
        let _client = ClientGuard::new(&self.clients);
        let Ok(mut write_half) = stream.try_clone() else {
            return;
        };
        let (tx, rx) = mpsc::channel::<Outgoing>();
        let connection = self.next_viewer.fetch_add(1, Ordering::SeqCst);
        let writer = thread::spawn(move || {
            for item in rx {
                match item {
                    Outgoing::Frame(frame) => {
                        if write_frame(&mut write_half, &frame).is_err() {
                            break;
                        }
                    }
                    Outgoing::Close => break,
                }
            }
            let _ = write_half.shutdown(Shutdown::Both);
        });
        let send = |response: Response| {
            let _ = tx.send(Outgoing::Frame(Frame::response(&response)));
        };

        let mut reader = BufReader::new(stream);
        let greeted = match read_frame(&mut reader) {
            Ok(Some(Frame::Control(bytes))) => match Request::parse(&bytes) {
                Ok(Request::Hello { protocol }) => {
                    let answer = answer_hello(protocol);
                    let welcomed = matches!(answer, Response::Welcome { .. });
                    send(answer);
                    welcomed
                }
                _ => {
                    send(Response::error(
                        ErrorCode::BadRequest,
                        "expected hello first",
                    ));
                    false
                }
            },
            _ => false,
        };

        let mut attached: Option<(Arc<Hosted>, u64)> = None;
        let mut stop_requested = false;
        loop {
            if !greeted {
                break;
            }
            let frame = match read_frame(&mut reader) {
                Ok(Some(frame)) => frame,
                Ok(None) => break,
                Err(error) => {
                    send(Response::error(ErrorCode::BadRequest, error.to_string()));
                    break;
                }
            };
            match frame {
                Frame::Input(bytes) => {
                    if let Some((hosted, _)) = &attached {
                        hosted.input(&bytes, now_ms());
                        self.refresh(hosted);
                    }
                }
                Frame::Output(_) => {
                    send(Response::error(
                        ErrorCode::BadRequest,
                        "clients cannot send output frames",
                    ));
                    break;
                }
                Frame::Control(bytes) => {
                    let request = match Request::parse(&bytes) {
                        Ok(request) => request,
                        Err(error) => {
                            send(Response::error(ErrorCode::BadRequest, error));
                            break;
                        }
                    };
                    match request {
                        Request::Hello { .. } => {
                            send(Response::error(ErrorCode::BadRequest, "already greeted"));
                        }
                        Request::List {} => {
                            self.wake_outside();
                            // Right after the host starts, the list would miss the
                            // owner's past conversations; wait for the first import.
                            self.wait_for_first_import(history::FIRST_IMPORT_WAIT);
                            send(self.list().unwrap_or_else(|error| *error));
                        }
                        Request::New {
                            agent,
                            cwd,
                            args,
                            term,
                            name,
                            group,
                        } => send(
                            self.create(agent, &cwd, args, term, name, group)
                                .map(|session| Response::Created { session })
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::Attach {
                            session,
                            cols,
                            rows,
                        } => {
                            if let Some((hosted, viewer)) = attached.take() {
                                hosted.detach(viewer);
                            }
                            match self.live_session(&session) {
                                Ok(hosted) => {
                                    let viewer = self.next_viewer.fetch_add(1, Ordering::SeqCst);
                                    match self.info(&hosted.id) {
                                        Ok(session) => send(Response::Attached { session }),
                                        Err(error) => {
                                            send(*error);
                                            continue;
                                        }
                                    }
                                    hosted.attach(Viewer {
                                        id: viewer,
                                        tx: tx.clone(),
                                        cols,
                                        rows,
                                    });
                                    self.refresh(&hosted);
                                    attached = Some((hosted, viewer));
                                }
                                Err(error) => send(*error),
                            }
                        }
                        Request::Resize { cols, rows } => {
                            if let Some((hosted, viewer)) = &attached {
                                hosted.resize(*viewer, cols, rows);
                            }
                        }
                        Request::Detach {} => {
                            if let Some((hosted, viewer)) = attached.take() {
                                hosted.detach(viewer);
                            }
                            send(Response::Detached);
                        }
                        Request::Rename { session, name } => send(
                            self.rename(&session, &name)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::Close { session } => send(
                            self.close(&session)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::Dismiss { sessions } => send(
                            self.dismiss(&sessions)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::Kill { session } => send(
                            self.kill(&session)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::Resume { session, term } => send(
                            self.resume(&session, term)
                                .map(|session| Response::Created { session })
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::Add { sessions } => send(
                            self.add_to_sidebar(&sessions)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::Remove { sessions } => send(
                            self.remove_from_sidebar(&sessions)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::NewGroup { name, session } => send(
                            self.new_group(name.as_deref(), session.as_deref())
                                .map(|group| Response::GroupCreated { group })
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::RenameGroup { group, name } => send(
                            self.rename_group(group, &name)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::DeleteGroup { group } => send(
                            self.delete_group(group)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::MoveGroup { group, before } => send(
                            self.move_group(group, before)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::CollapseGroup { group, collapsed } => send(
                            self.collapse_group(group, collapsed)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::SetGroup { session, group } => send(
                            self.set_group(&session, group)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::Seen { session } => send(
                            self.seen(&session)
                                .map(|()| Response::Done)
                                .unwrap_or_else(|error| *error),
                        ),
                        Request::Subscribe { usage } => {
                            lock(&self.subscribers).push(Subscriber {
                                connection,
                                tx: tx.clone(),
                                usage,
                                follows: None,
                                visible: true,
                            });
                            self.wake_outside();
                            send(Response::Done);
                        }
                        Request::Visible { visible } => {
                            for subscriber in lock(&self.subscribers).iter_mut() {
                                if subscriber.connection == connection {
                                    subscriber.visible = visible;
                                }
                            }
                            if visible {
                                self.wake_outside();
                            }
                            send(Response::Done);
                        }
                        Request::FollowDetails { session } => {
                            self.follow_details(connection, session);
                            send(Response::Done);
                        }
                        Request::Usage {} => send(Response::Usage {
                            agents: self.usage(),
                        }),
                        Request::RefreshUsage {} => {
                            self.refresh_usage();
                            send(Response::Done);
                        }
                        Request::Stop {} => {
                            send(Response::Done);
                            stop_requested = true;
                            break;
                        }
                    }
                }
            }
        }

        if let Some((hosted, viewer)) = attached {
            hosted.detach(viewer);
        }
        // A client that is gone no longer counts as watching, so usage reads stop now
        // rather than after the next failed send.
        lock(&self.subscribers).retain(|subscriber| subscriber.connection != connection);
        let _ = tx.send(Outgoing::Close);
        drop(tx);
        let _ = writer.join();
        if stop_requested {
            self.shutdown("stop requested");
        }
    }

    // ---- requests --------------------------------------------------------------

    fn list(&self) -> OpResult<Response> {
        let rows = lock(&self.store).list().map_err(storage_error)?;
        let mut sessions: Vec<SessionInfo> =
            rows.into_iter().map(|row| self.describe(row)).collect();
        sessions.extend(lock(&self.outside).unlinked.iter().cloned());
        Ok(Response::Sessions {
            sessions,
            groups: self.groups()?,
        })
    }

    fn info(&self, id: &str) -> OpResult<SessionInfo> {
        let row = lock(&self.store)
            .get(id)
            .map_err(storage_error)?
            .ok_or_else(|| not_found(id))?;
        Ok(self.describe(row))
    }

    /// A row as clients see it: plus whether it runs here, the agent's title (kept in
    /// memory only), and any outside process running its conversation.
    fn describe(&self, row: SessionRow) -> SessionInfo {
        let live = lock(&self.live).contains_key(&row.id);
        let title = row.agent_session_id.as_ref().and_then(|conversation| {
            lock(&self.titles)
                .get(&(row.agent, conversation.clone()))
                .cloned()
        });
        let external = lock(&self.outside).linked.get(&row.id).cloned();
        SessionInfo {
            id: row.id,
            agent: row.agent,
            cwd: row.cwd,
            name: row.name,
            agent_session_id: row.agent_session_id,
            status: row.status,
            needs_reason: row.needs_reason,
            created_at: row.created_at,
            last_activity_at: row.last_activity_at,
            ended_at: row.ended_at,
            live,
            origin: match row.origin {
                Origin::Hosted => hq_protocol::Origin::Hosted,
                Origin::History => hq_protocol::Origin::History,
            },
            title,
            external,
            in_sidebar: row.sidebar_added_at.is_some(),
            group: row.group_id,
            owner_named: row.owner_named,
            interrupted: row.interrupted_at.is_some(),
        }
    }

    fn create(
        self: &Arc<Self>,
        agent: Agent,
        cwd: &str,
        args: Vec<String>,
        term: Option<String>,
        name: Option<String>,
        group: Option<i64>,
    ) -> OpResult<SessionInfo> {
        self.refuse_while_stopping()?;
        if let Some(group) = group {
            self.group_row(group)?;
        }
        let folder = canonical_folder(cwd)?;
        let program = self.resolve_agent(agent)?;
        let id = uuid::Uuid::new_v4().to_string();
        let (name, owner_named) = match name.map(|name| name.trim().to_owned()) {
            Some(name) if !name.is_empty() => {
                self.ensure_name_free(&name)?;
                (name, true)
            }
            _ => (
                lock(&self.store)
                    .next_default_name(agent)
                    .map_err(storage_error)?,
                false,
            ),
        };
        let spawned = self.spawn(SpawnPlan {
            id: &id,
            agent,
            folder: &folder,
            program,
            conversation: Conversation::New { id: &id },
            extra_args: args,
            term,
        })?;
        let now = now_ms();
        let row = SessionRow {
            id: id.clone(),
            agent,
            cwd: folder,
            name,
            // Claude takes the host's id as its own session id (`--session-id`).
            agent_session_id: (agent == Agent::Claude).then(|| id.clone()),
            status: Status::Done,
            needs_reason: None,
            created_at: now,
            last_activity_at: now,
            ended_at: None,
            origin: Origin::Hosted,
            // Sessions started in Headquarter join the sidebar on their own.
            sidebar_added_at: Some(now),
            group_id: group,
            owner_named,
            interrupted_at: None,
        };
        if let Err(error) = lock(&self.store).insert(&row) {
            spawned.hosted.kill();
            return Err(storage_error(error));
        }
        self.start(spawned);
        self.log.line(&format!(
            "session started id={id} agent={agent} resume=false"
        ));
        self.broadcast(&id);
        Ok(self.describe(row))
    }

    fn resume(self: &Arc<Self>, reference: &str, term: Option<String>) -> OpResult<SessionInfo> {
        self.refuse_while_stopping()?;
        let row = self.resolve(reference)?;
        if lock(&self.live).contains_key(&row.id) {
            return Err(fail(
                ErrorCode::AlreadyLive,
                format!("session `{}` is already running; attach to it", row.name),
            ));
        }
        if let Some(external) = lock(&self.outside).linked.get(&row.id) {
            let pid = external
                .pid
                .map_or_else(|| "unknown".to_owned(), |pid| pid.to_string());
            return Err(fail(
                ErrorCode::ExternalLive,
                format!(
                    "session `{}` is running in another terminal (pid {pid}); close it there first",
                    row.name
                ),
            ));
        }
        let folder = canonical_folder(&row.cwd)?;
        let program = self.resolve_agent(row.agent)?;
        let spawned = self.spawn(SpawnPlan {
            id: &row.id,
            agent: row.agent,
            folder: &folder,
            program,
            conversation: Conversation::Resume(row.agent_session_id.as_deref()),
            extra_args: Vec::new(),
            term,
        })?;
        // A session resumed in Headquarter is in use there, so it joins the sidebar.
        let now = now_ms();
        let stored = {
            let store = lock(&self.store);
            store
                .reactivate(&row.id)
                .and_then(|()| store.add_to_sidebar(std::slice::from_ref(&row.id), now))
        };
        if let Err(error) = stored {
            spawned.hosted.kill();
            return Err(storage_error(error));
        }
        self.start(spawned);
        self.log.line(&format!(
            "session started id={} agent={} resume=true known_conversation={}",
            row.id,
            row.agent,
            row.agent_session_id.is_some()
        ));
        self.broadcast(&row.id);
        self.info(&row.id)
    }

    /// The executable for `agent`: the settings file's pin, else discovery across the
    /// process `PATH`, the login-shell `PATH`, and the platform's install folders.
    fn resolve_agent(&self, agent: Agent) -> OpResult<PathBuf> {
        let settings = lock(&self.settings)
            .current()
            .map_err(|error| fail(ErrorCode::Settings, error.to_string()))?;
        if !settings.agent_enabled(agent.id()) {
            return Err(fail(
                ErrorCode::AgentDisabled,
                format!("`{agent}` is turned off in the settings file"),
            ));
        }
        let Some(known) = hq_discovery::known(agent.id()) else {
            return Err(fail(
                ErrorCode::AgentNotFound,
                format!("`{agent}` is not a known agent"),
            ));
        };
        let context = SearchContext::current(self.platform.clone(), lock(&self.login_path).clone());
        let report = hq_discovery::search(&context, known, settings.agent_path(agent.id()));
        if let Some(problem) = report.pin_problem {
            return Err(fail(ErrorCode::AgentNotFound, problem));
        }
        report.used_path().map(Path::to_owned).ok_or_else(|| {
            const SHOWN: usize = 8;
            let mut searched: Vec<String> = report
                .searched
                .iter()
                .take(SHOWN)
                .map(|folder| folder.display().to_string())
                .collect();
            if report.searched.len() > SHOWN {
                searched.push(format!("and {} more", report.searched.len() - SHOWN));
            }
            fail(
                ErrorCode::AgentNotFound,
                format!(
                    "`{}` was not found. Searched: {}",
                    agent.binary(),
                    searched.join(", ")
                ),
            )
        })
    }

    /// Claude Code's executable for the outside scan. Found like an agent to host, but
    /// without the enabled check: a turned-off agent can still run elsewhere.
    fn locate_claude(&self) -> Option<PathBuf> {
        let pinned = lock(&self.settings)
            .current()
            .ok()
            .and_then(|settings| settings.agent_path("claude").map(Path::to_owned));
        let context = SearchContext::current(self.platform.clone(), lock(&self.login_path).clone());
        hq_discovery::search(&context, hq_discovery::known("claude")?, pinned.as_deref())
            .used_path()
            .map(Path::to_owned)
    }

    /// A desktop-launched host lacks the shell's `PATH`, so learn it once.
    fn capture_login_path(self: Arc<Self>) {
        let allowed = lock(&self.settings)
            .current()
            .map_or(true, |settings| settings.login_shell);
        if !allowed {
            self.log.line("login shell PATH skipped by settings");
            return;
        }
        let path = hq_discovery::shell::login_shell_path(hq_discovery::shell::LOGIN_SHELL_TIMEOUT);
        self.log.line(&format!(
            "login shell PATH {}",
            if path.is_some() {
                "captured"
            } else {
                "unavailable"
            }
        ));
        *lock(&self.login_path) = path;
    }

    fn ensure_name_free(&self, name: &str) -> OpResult<()> {
        if lock(&self.store).name_taken(name).map_err(storage_error)? {
            return Err(fail(
                ErrorCode::BadRequest,
                format!("the name `{name}` is already used by another session"),
            ));
        }
        Ok(())
    }

    fn refuse_while_stopping(&self) -> OpResult<()> {
        if self.stopping.load(Ordering::SeqCst) {
            return Err(fail(
                ErrorCode::Stopping,
                "the Headquarter host is stopping; try again in a moment",
            ));
        }
        Ok(())
    }

    /// Renames a session. A blank name gives back a generated one, so the agent's own
    /// title shows again.
    fn rename(&self, reference: &str, name: &str) -> OpResult<()> {
        let name = name.trim();
        let row = self.resolve(reference)?;
        if name.is_empty() {
            if !row.owner_named {
                return Ok(());
            }
            let store = lock(&self.store);
            let generated = store.next_default_name(row.agent).map_err(storage_error)?;
            store
                .rename(&row.id, &generated, false)
                .map_err(storage_error)?;
        } else {
            if row.name == name && row.owner_named {
                return Ok(());
            }
            if row.name != name {
                self.ensure_name_free(name)?;
            }
            lock(&self.store)
                .rename(&row.id, name, true)
                .map_err(storage_error)?;
        }
        self.broadcast(&row.id);
        Ok(())
    }

    /// All or nothing: every reference is checked before the sidebar changes.
    fn add_to_sidebar(&self, references: &[String]) -> OpResult<()> {
        let unlinked = lock(&self.outside).unlinked.clone();
        let ids = self.resolve_all(references, |reference| {
            match unlinked.iter().find(|session| session.id == reference) {
                Some(outside) => Err(fail(
                    ErrorCode::NotAddable,
                    format!(
                        "`{}` runs in another terminal with no known conversation yet; it can be added once its conversation is known",
                        outside.name
                    ),
                )),
                None => self.resolve(reference).map(|row| row.id),
            }
        })?;
        lock(&self.store)
            .add_to_sidebar(&ids, now_ms())
            .map_err(storage_error)?;
        for id in &ids {
            self.broadcast(id);
        }
        Ok(())
    }

    /// Clears interrupted marks without restarting: of `references`, or of every
    /// session when there are none (session-restore D1).
    fn dismiss(&self, references: &[String]) -> OpResult<()> {
        let ids: Vec<String> = if references.is_empty() {
            lock(&self.store)
                .list()
                .map_err(storage_error)?
                .into_iter()
                .filter(|row| row.interrupted_at.is_some())
                .map(|row| row.id)
                .collect()
        } else {
            self.resolve_all(references, |reference| {
                self.resolve(reference).map(|row| row.id)
            })?
        };
        lock(&self.store)
            .dismiss_interrupted(&ids)
            .map_err(storage_error)?;
        for id in &ids {
            self.broadcast(id);
        }
        Ok(())
    }

    fn remove_from_sidebar(&self, references: &[String]) -> OpResult<()> {
        let ids = self.resolve_all(references, |reference| {
            self.resolve(reference).map(|row| row.id)
        })?;
        lock(&self.store)
            .remove_from_sidebar(&ids)
            .map_err(storage_error)?;
        for id in &ids {
            self.broadcast(id);
        }
        Ok(())
    }

    /// Resolves every reference, or fails naming each one that does not resolve.
    fn resolve_all(
        &self,
        references: &[String],
        resolve: impl Fn(&str) -> OpResult<String>,
    ) -> OpResult<Vec<String>> {
        if references.is_empty() {
            return Err(fail(ErrorCode::BadRequest, "name at least one session"));
        }
        let mut ids = Vec::new();
        let mut problems: Vec<(ErrorCode, String)> = Vec::new();
        for reference in references {
            match resolve(reference) {
                Ok(id) => ids.push(id),
                Err(error) => match *error {
                    Response::Error { code, message } => problems.push((code, message)),
                    other => problems.push((ErrorCode::BadRequest, format!("{other:?}"))),
                },
            }
        }
        match problems.first() {
            None => Ok(ids),
            Some((code, _)) => {
                let messages: Vec<String> = problems
                    .iter()
                    .map(|(_, message)| message.clone())
                    .collect();
                Err(fail(*code, messages.join("; ")))
            }
        }
    }

    // ---- groups ----------------------------------------------------------------

    fn groups(&self) -> OpResult<Vec<GroupInfo>> {
        let rows = lock(&self.store).groups().map_err(storage_error)?;
        Ok(rows
            .into_iter()
            .map(|row| GroupInfo {
                id: row.id,
                name: row.name,
                collapsed: row.collapsed,
            })
            .collect())
    }

    fn group_row(&self, id: i64) -> OpResult<hq_store::GroupRow> {
        lock(&self.store)
            .group(id)
            .map_err(storage_error)?
            .ok_or_else(|| fail(ErrorCode::NotFound, format!("no group has id {id}")))
    }

    fn new_group(&self, name: Option<&str>, session: Option<&str>) -> OpResult<GroupInfo> {
        let member = session
            .map(|reference| self.sidebar_member(reference))
            .transpose()?;
        let name = match name {
            Some(name) => hq_protocol::group_name(name).map_err(bad_request)?,
            None => {
                let existing = self.groups()?;
                hq_protocol::default_group_name(existing.iter().map(|group| group.name.as_str()))
            }
        };
        let id = {
            let store = lock(&self.store);
            let id = store.create_group(&name, now_ms()).map_err(storage_error)?;
            if let Some(member) = &member {
                store
                    .set_session_group(member, Some(id))
                    .map_err(storage_error)?;
            }
            id
        };
        self.broadcast_groups()?;
        if let Some(member) = &member {
            self.broadcast(member);
        }
        Ok(GroupInfo {
            id,
            name,
            collapsed: false,
        })
    }

    fn rename_group(&self, id: i64, name: &str) -> OpResult<()> {
        self.group_row(id)?;
        let name = hq_protocol::group_name(name).map_err(bad_request)?;
        lock(&self.store)
            .rename_group(id, &name)
            .map_err(storage_error)?;
        self.broadcast_groups()
    }

    /// Deletes the group; its sessions stay in the sidebar, in no group.
    fn delete_group(&self, id: i64) -> OpResult<()> {
        self.group_row(id)?;
        let members: Vec<String> = {
            let store = lock(&self.store);
            let members = store
                .list()
                .map_err(storage_error)?
                .into_iter()
                .filter(|row| row.group_id == Some(id))
                .map(|row| row.id)
                .collect();
            store.delete_group(id).map_err(storage_error)?;
            members
        };
        self.broadcast_groups()?;
        for member in &members {
            self.broadcast(member);
        }
        Ok(())
    }

    fn move_group(&self, id: i64, before: Option<i64>) -> OpResult<()> {
        self.group_row(id)?;
        if let Some(before) = before {
            self.group_row(before)?;
        }
        lock(&self.store)
            .move_group(id, before)
            .map_err(storage_error)?;
        self.broadcast_groups()
    }

    fn collapse_group(&self, id: i64, collapsed: bool) -> OpResult<()> {
        self.group_row(id)?;
        lock(&self.store)
            .set_group_collapsed(id, collapsed)
            .map_err(storage_error)?;
        self.broadcast_groups()
    }

    fn set_group(&self, reference: &str, group: Option<i64>) -> OpResult<()> {
        let member = self.sidebar_member(reference)?;
        if let Some(group) = group {
            self.group_row(group)?;
        }
        lock(&self.store)
            .set_session_group(&member, group)
            .map_err(storage_error)?;
        self.broadcast(&member);
        Ok(())
    }

    /// The id of a session in the sidebar; groups hold sidebar sessions only.
    fn sidebar_member(&self, reference: &str) -> OpResult<String> {
        let row = self.resolve(reference)?;
        if row.sidebar_added_at.is_none() {
            return Err(fail(
                ErrorCode::BadRequest,
                format!("`{}` is not in the sidebar; add it first", row.name),
            ));
        }
        Ok(row.id)
    }

    fn broadcast_groups(&self) -> OpResult<()> {
        let frame = Frame::response(&Response::GroupsChanged {
            groups: self.groups()?,
        });
        lock(&self.subscribers)
            .retain(|subscriber| subscriber.tx.send(Outgoing::Frame(frame.clone())).is_ok());
        Ok(())
    }

    /// Clears "done, not read yet" for a running session; anything else is unchanged.
    fn seen(&self, reference: &str) -> OpResult<()> {
        let row = self.resolve(reference)?;
        let hosted = lock(&self.live).get(&row.id).cloned();
        if let Some(hosted) = hosted {
            // Seeing a session reads it; it is not asking the agent for anything.
            hosted.mark_viewed();
            self.refresh(&hosted);
        }
        Ok(())
    }

    fn kill(&self, reference: &str) -> OpResult<()> {
        let hosted = self.live_session(reference)?;
        hosted.kill();
        Ok(())
    }

    /// A session by exact id, unique name, or unique id prefix.
    fn resolve(&self, reference: &str) -> OpResult<SessionRow> {
        let rows = lock(&self.store).list().map_err(storage_error)?;
        if let Some(row) = rows.iter().find(|row| row.id == reference) {
            return Ok(row.clone());
        }
        let by_name: Vec<&SessionRow> = rows.iter().filter(|row| row.name == reference).collect();
        let candidates = if by_name.is_empty() && reference.len() >= MIN_ID_PREFIX {
            rows.iter()
                .filter(|row| row.id.starts_with(reference))
                .collect()
        } else {
            by_name
        };
        match candidates.as_slice() {
            [only] => Ok((*only).clone()),
            [] => Err(not_found(reference)),
            several => Err(fail(
                ErrorCode::Ambiguous,
                format!(
                    "`{reference}` matches {} sessions; use more of the id",
                    several.len()
                ),
            )),
        }
    }

    fn live_session(&self, reference: &str) -> OpResult<Arc<Hosted>> {
        let row = self.resolve(reference)?;
        lock(&self.live).get(&row.id).cloned().ok_or_else(|| {
            fail(
                ErrorCode::NotLive,
                format!(
                    "session `{}` is not running; use `hq resume {}`",
                    row.name, row.name
                ),
            )
        })
    }

    // ---- hosting ---------------------------------------------------------------

    fn spawn(&self, plan: SpawnPlan<'_>) -> OpResult<Spawned> {
        let failed = |what: &str, error: &dyn std::fmt::Display| {
            fail(ErrorCode::SpawnFailed, format!("{what}: {error}"))
        };
        let owner = OwnerSettings::discover(Path::new(plan.folder));
        let mut spec = adapters::launch_spec(
            plan.agent,
            &LaunchContext {
                helper_exe: &self.helper_exe,
                hook_socket: &self.paths.hook_socket,
                conversation: plan.conversation,
            },
            &owner,
        );
        spec.args.extend(plan.extra_args);

        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| failed("cannot open a pseudo-terminal", &error))?;
        let mut command = CommandBuilder::new(&plan.program);
        command.args(&spec.args);
        command.cwd(plan.folder);
        for marker in adapters::PARENT_SESSION_MARKERS {
            command.env_remove(marker);
        }
        command.env(SESSION_ENV, plan.id);
        command.env("TERM", plan.term.as_deref().unwrap_or("xterm-256color"));
        command.env("COLORTERM", "truecolor");
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| failed(&format!("cannot start {}", plan.agent), &error))?;
        drop(pair.slave);
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| failed("pty reader", &error))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| failed("pty writer", &error))?;
        let now = now_ms();
        let hosted = Arc::new(Hosted::new(HostedParts {
            id: plan.id.to_owned(),
            agent: plan.agent,
            cwd: plan.folder.to_owned(),
            started_at_ms: now,
            writer,
            master: pair.master,
            killer: child.clone_killer(),
            pid: child.process_id(),
            tracker: SessionTracker::new(spec.structured, Thresholds::default(), now),
            conversation_known: match plan.conversation {
                Conversation::New { .. } => plan.agent == Agent::Claude,
                Conversation::Resume(id) => id.is_some(),
            },
        }));
        Ok(Spawned {
            hosted,
            reader,
            child,
        })
    }

    fn start(self: &Arc<Self>, spawned: Spawned) {
        let Spawned {
            hosted,
            reader,
            mut child,
        } = spawned;
        lock(&self.live).insert(hosted.id.clone(), Arc::clone(&hosted));
        {
            let host = Arc::clone(self);
            let hosted = Arc::clone(&hosted);
            thread::spawn(move || host.pump_output(&hosted, reader));
        }
        let host = Arc::clone(self);
        thread::spawn(move || {
            let exit_code = child.wait().ok().map(|status| status.exit_code());
            host.on_exit(&hosted, exit_code);
        });
    }

    fn pump_output(&self, hosted: &Arc<Hosted>, mut reader: Box<dyn Read + Send>) {
        let mut scanner = Scanner::new();
        let mut buffer = [0u8; 16 * 1024];
        let mut last_activity_ms = 0;
        loop {
            let count = match reader.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(count) => count,
            };
            let chunk = &buffer[..count];
            let signals = scanner.feed(chunk);
            hosted.output(chunk);
            let now = now_ms();
            for signal in signals {
                if signal == TerminalSignal::ProgramStatusQuery {
                    hosted.write_to_agent(PROGRAM_STATUS_SUPPORT_REPLY);
                } else if let Some((source, event)) =
                    adapters::terminal_event(hosted.agent, &signal)
                {
                    if source == Source::ProgramStatus {
                        hosted.apply_program_status(event, now);
                    } else {
                        hosted.apply(event, now);
                    }
                }
            }
            if now.saturating_sub(last_activity_ms) >= OUTPUT_ACTIVITY_MS {
                last_activity_ms = now;
                hosted.apply(EventKind::OutputActivity, now);
            }
            self.refresh(hosted);
        }
    }

    fn on_exit(&self, hosted: &Arc<Hosted>, exit_code: Option<u32>) {
        let now = now_ms();
        hosted.apply(EventKind::Ended, now);
        lock(&self.live).remove(&hosted.id);
        self.forget_details(&hosted.id);
        if let Err(error) = lock(&self.store).end(&hosted.id, now) {
            self.log.line(&format!(
                "index update failed id={} error={error}",
                hosted.id
            ));
        }
        // The owner quit the agent, here or from inside it: the session leaves the
        // sidebar. A crash, a signal or a host stop keeps it, as not running.
        let quit = exit_code == Some(0) && !self.stopping.load(Ordering::SeqCst);
        if (hosted.closing() || quit)
            && let Err(error) =
                lock(&self.store).remove_from_sidebar(std::slice::from_ref(&hosted.id))
        {
            self.log.line(&format!(
                "index update failed id={} error={error}",
                hosted.id
            ));
        }
        hosted.end_viewers(exit_code);
        self.log.line(&format!(
            "session ended id={} exit_code={}",
            hosted.id,
            exit_code.map_or_else(|| "unknown".to_owned(), |code| code.to_string())
        ));
        self.broadcast(&hosted.id);
    }

    /// Persists and announces a status change, and looks up the agent's own
    /// conversation id after its first completed turn.
    fn refresh(&self, hosted: &Arc<Hosted>) {
        let now = now_ms();
        let Some(change) = hosted.status_change() else {
            return;
        };
        let status = change.status;
        let activity = change.is_activity().then_some(now);
        if let Err(error) =
            lock(&self.store).update_status(&hosted.id, status, change.reason, activity)
        {
            self.log.line(&format!(
                "index update failed id={} error={error}",
                hosted.id
            ));
        }
        if status == Status::DoneUnread {
            self.resolve_conversation(hosted);
        }
        if hosted.agent == Agent::Codex
            && change.previous == Some(Status::Working)
            && matches!(status, Status::DoneUnread | Status::Done)
        {
            self.codex_turn_ended();
        }
        self.broadcast(&hosted.id);
    }

    /// Looked up after each completed turn until found: an agent writes its
    /// conversation file only once the first turn has started.
    fn resolve_conversation(&self, hosted: &Arc<Hosted>) {
        if *lock(&hosted.conversation_known) {
            return;
        }
        let started_secs = hosted.started_at_ms / 1000;
        let found = match hosted.agent {
            Agent::Codex => external::codex_conversation(&hosted.cwd, started_secs),
            Agent::Agy => external::agy_conversation_since(started_secs),
            Agent::Claude => None,
        };
        match found {
            Some(conversation) => {
                *lock(&hosted.conversation_known) = true;
                self.drop_imported_duplicate(hosted, &conversation);
                if let Err(error) =
                    lock(&self.store).set_agent_session_id(&hosted.id, &conversation)
                {
                    self.log.line(&format!(
                        "index update failed id={} error={error}",
                        hosted.id
                    ));
                }
                self.log
                    .line(&format!("conversation found id={}", hosted.id));
            }
            None => self
                .log
                .line(&format!("conversation not found id={}", hosted.id)),
        }
    }

    fn broadcast(&self, id: &str) {
        let Ok(session) = self.info(id) else {
            return;
        };
        let frame = Frame::response(&Response::SessionChanged { session });
        lock(&self.subscribers)
            .retain(|subscriber| subscriber.tx.send(Outgoing::Frame(frame.clone())).is_ok());
    }

    // ---- hooks, time, signals, shutdown ---------------------------------------

    fn serve_hooks(self: Arc<Self>, listener: UnixListener) {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let host = Arc::clone(&self);
            thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    host.hook_line(&line);
                }
            });
        }
    }

    fn hook_line(&self, line: &str) {
        let Ok(payload) = serde_json::from_str::<ForwardedPayload>(line) else {
            self.log.line("hook payload unreadable");
            return;
        };
        if payload.agent == hq_signals::helpers::DETAILS_KIND {
            if let Some(session) = &payload.session {
                self.details_reported(session, &payload.raw);
            }
            return;
        }
        let Some(agent) = Agent::from_id(&payload.agent) else {
            self.log.line("hook payload for an unknown agent");
            return;
        };
        let now = now_ms();
        let record = match hook::normalize(agent, &payload.raw, now) {
            Ok(Some(record)) => record,
            Ok(None) => return,
            Err(error) => {
                self.log.line(&format!("hook payload rejected: {error}"));
                return;
            }
        };
        let hosted = {
            let live = lock(&self.live);
            payload
                .session
                .as_deref()
                .and_then(|id| live.get(id).cloned())
                .or_else(|| {
                    record
                        .session_id
                        .as_deref()
                        .and_then(|id| live.get(id).cloned())
                })
        };
        let Some(hosted) = hosted else {
            self.log.line("hook payload for no running session");
            return;
        };
        hosted.apply_hook(record.event, now);
        self.refresh(&hosted);
    }

    fn tick_loop(self: Arc<Self>) {
        let mut idle_since: Option<Instant> = None;
        loop {
            thread::sleep(TICK);
            let sessions: Vec<Arc<Hosted>> = lock(&self.live).values().cloned().collect();
            let now = now_ms();
            for hosted in &sessions {
                hosted.tick(now);
                self.refresh(hosted);
            }
            if sessions.is_empty() && self.clients.load(Ordering::SeqCst) == 0 {
                let since = *idle_since.get_or_insert_with(Instant::now);
                if since.elapsed() >= self.idle_exit {
                    self.shutdown("idle");
                }
            } else {
                idle_since = None;
            }
        }
    }

    fn spawn_signal_handler(self: &Arc<Self>) {
        use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
        let Ok(mut signals) = signal_hook::iterator::Signals::new([SIGHUP, SIGINT, SIGTERM]) else {
            self.log.line("signal handler unavailable");
            return;
        };
        let host = Arc::clone(self);
        thread::spawn(move || {
            for signal in signals.forever() {
                // A hangup of the terminal that started the host is not a reason to stop.
                if signal != SIGHUP {
                    host.shutdown("signal");
                }
            }
        });
    }

    /// Ends every hosted agent, records them inactive, and exits the process.
    fn shutdown(&self, reason: &str) -> ! {
        if self.stopping.swap(true, Ordering::SeqCst) {
            // Another thread is already shutting down and will exit the process.
            loop {
                thread::park();
            }
        }
        self.log.line(&format!("host stopping reason={reason}"));
        // New clients reach a fresh host instead of this one while agents wind down.
        let _ = std::fs::remove_file(&self.paths.control_socket);
        let _ = std::fs::remove_file(&self.paths.hook_socket);
        let sessions: Vec<Arc<Hosted>> = lock(&self.live).values().cloned().collect();
        for hosted in &sessions {
            hosted.kill();
        }
        let deadline = Instant::now() + STOP_GRACE;
        while !lock(&self.live).is_empty() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(50));
        }
        if let Err(error) = lock(&self.store).end_all_live(now_ms()) {
            self.log.line(&format!("index update failed error={error}"));
        }
        self.log.line("host stopped");
        std::process::exit(0);
    }
}

fn not_found(reference: &str) -> Box<Response> {
    fail(
        ErrorCode::NotFound,
        format!("no session matches `{reference}`"),
    )
}

fn canonical_folder(cwd: &str) -> OpResult<String> {
    let missing = || {
        fail(
            ErrorCode::FolderNotFound,
            format!("folder `{cwd}` does not exist"),
        )
    };
    let path = Path::new(cwd).canonicalize().map_err(|_| missing())?;
    if !path.is_dir() {
        return Err(missing());
    }
    Ok(path.to_string_lossy().into_owned())
}
