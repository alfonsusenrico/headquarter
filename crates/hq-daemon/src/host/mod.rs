//! The host process: one per user, owning every hosted session, the session index,
//! and the control and hook sockets.

mod claude_usage;
mod close;
mod details;
mod history;
mod outside;
mod remote;
mod remove;
mod starting;
mod usage;

use std::collections::{HashMap, HashSet};
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read};
use std::net::Shutdown;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, mpsc};
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
use crate::session::{Hosted, HostedParts, Outgoing, Report, Viewer};

const TICK: Duration = Duration::from_millis(250);
/// Output activity is fed to the status machine at most this often.
const OUTPUT_ACTIVITY_MS: u64 = 200;
/// How long `stop` waits for agents to exit after SIGHUP before leaving.
const STOP_GRACE: Duration = Duration::from_secs(3);
/// How long a new host waits for a stopping one to exit: its agents get `STOP_GRACE`,
/// and ending many of them takes longer.
const STOPPING_HOST_WAIT: Duration = Duration::from_secs(10);
/// Shortest id prefix accepted as a session reference.
const MIN_ID_PREFIX: usize = 4;
/// How long one answer of `claude agents --json` serves resumes (fix-claude-conversations D3).
const CLAUDE_RUNNING_REUSE: Duration = Duration::from_secs(3);
/// How long a stopped background conversation may take to leave Claude Code's list.
const BACKGROUND_STOP_WAIT: Duration = Duration::from_secs(8);
/// Tells programs that check it (Node's `supports-hyperlinks`, which Claude Code uses)
/// that the terminal opens links they mark up (OSC 8), so a link an agent wraps over
/// several lines still opens whole (add-terminal-links).
const LINKS_ENV: (&str, &str) = ("FORCE_HYPERLINK", "1");

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
    /// Where removed Claude transcripts go instead of the system Trash (tests only).
    pub trash_dir: Option<PathBuf>,
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
    take_lock(&lock, &paths.control_socket)?;

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
        trash_dir: options.trash_dir,
        store: Mutex::new(store),
        live: Mutex::new(HashMap::new()),
        shells: Mutex::new(HashMap::new()),
        starts: starting::Starts::default(),
        subscribers: Mutex::new(Vec::new()),
        usage: Mutex::new(usage::UsageBook::default()),
        usage_settled: Condvar::new(),
        details: Mutex::new(details::DetailsBook::default()),
        clients: AtomicUsize::new(0),
        next_viewer: AtomicU64::new(1),
        stopping: AtomicBool::new(false),
        history: Mutex::new(HistoryReader::new(HistoryRoots::standard())),
        titles: Mutex::new(HashMap::new()),
        offered: Mutex::new(HashSet::new()),
        agy_conversations: Mutex::new(Vec::new()),
        claude_running: Mutex::new(None),
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
        remote: remote::Remote::default(),
        platform: hq_discovery::platform::detect(),
        login_path: Mutex::new(None),
        login_path_settled: AtomicBool::new(false),
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
    {
        let host = Arc::clone(&host);
        thread::spawn(move || host.start_remote_if_enabled());
    }
    {
        let order = host.queue_starts();
        let host = Arc::clone(&host);
        thread::spawn(move || host.start_with_host(&order));
    }
    for stream in control.incoming() {
        let Ok(stream) = stream else { continue };
        let host = Arc::clone(&host);
        thread::spawn(move || host.serve(stream));
    }
    Ok(())
}

/// Takes the single-instance lock. A running host keeps its control socket; one that is
/// stopping removes it first and holds the lock until its agents have ended, so a host
/// started meanwhile (an update's restart, `hq` right after `hq stop`) waits for it
/// instead of giving up (fix-host-start-after-stop).
fn take_lock(lock: &std::fs::File, control_socket: &Path) -> Result<(), HostError> {
    let deadline = Instant::now() + STOPPING_HOST_WAIT;
    loop {
        match lock.try_lock() {
            Ok(()) => return Ok(()),
            Err(std::fs::TryLockError::WouldBlock) => {
                if control_socket.exists() || Instant::now() >= deadline {
                    return Err(HostError::AlreadyRunning);
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(std::fs::TryLockError::Error(error)) => {
                return Err(HostError::Io("lock file", error));
            }
        }
    }
}

fn bind_private(path: &Path) -> Result<UnixListener, HostError> {
    let _ = std::fs::remove_file(path);
    let listener = UnixListener::bind(path).map_err(|error| HostError::Io("socket", error))?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|error| HostError::Io("socket permissions", error))?;
    Ok(listener)
}

/// Whether the agent has saved conversation `id`, so it can resume it. Only Claude Code
/// is checked: Headquarter gives it the id before anything is saved. Others count as
/// saved, and an unknown home folder too, so their resume works as before.
fn conversation_saved(agent: Agent, id: Option<&str>) -> bool {
    let (Agent::Claude, Some(id)) = (agent, id) else {
        return true;
    };
    let Some(projects) = claude_projects() else {
        return true;
    };
    hq_signals::history::claude_transcript(&projects, id).is_some()
}

/// Claude Code's saved conversations, one folder per project.
fn claude_projects() -> Option<PathBuf> {
    hq_signals::paths::claude_config_dir().map(|dir| dir.join("projects"))
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
    trash_dir: Option<PathBuf>,
    store: Mutex<Store>,
    live: Mutex<HashMap<String, Arc<Hosted>>>,
    /// Each session's shell, by session id; in memory only (add-session-shell-panel).
    shells: Mutex<HashMap<String, Arc<Hosted>>>,
    /// Sessions starting with the host, and why any could not (add-inactive-sessions).
    starts: starting::Starts,
    subscribers: Mutex<Vec<Subscriber>>,
    usage: Mutex<usage::UsageBook>,
    /// Signalled when a usage read finishes and its result has gone out.
    usage_settled: Condvar,
    /// Sessions' details, in memory only.
    details: Mutex<details::DetailsBook>,
    clients: AtomicUsize,
    next_viewer: AtomicU64,
    stopping: AtomicBool,
    log: Log,
    history: Mutex<HistoryReader>,
    /// Agent titles by (agent, conversation id), for display only; never stored.
    titles: Mutex<HashMap<(Agent, String), String>>,
    /// The past conversations Add sessions offers, as the last history pass found them:
    /// still kept by their agent, and started by the owner (offer-continuable-sessions).
    offered: Mutex<HashSet<(Agent, String)>>,
    /// agy's top-level conversations as the last history pass read them, for linking
    /// agy processes in other terminals.
    agy_conversations: Mutex<Vec<outside::AgyConversation>>,
    /// Claude Code's running conversations, and when they were asked (`claude_running`).
    claude_running: Mutex<Option<(Instant, Vec<external::ExternalSession>)>>,
    outside: Mutex<outside::OutsideView>,
    outside_wake: outside::OutsideWake,
    first_import: history::FirstImport,
    settings: Mutex<SettingsWatcher>,
    /// Remote control's gateway, while it runs (add-remote-gateway).
    remote: remote::Remote,
    platform: Platform,
    /// The owner's login-shell `PATH`, captured once in the background at startup.
    login_path: Mutex<Option<String>>,
    /// Set once the login-shell `PATH` is known, unavailable, or turned off; agents
    /// started earlier may not find what they need (`hq_discovery::shell::agent_path`).
    login_path_settled: AtomicBool,
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
                    if let Some((hosted, viewer)) = &attached {
                        hosted.input_from(*viewer, &bytes, now_ms());
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
                        Request::AttachShell {
                            session,
                            cols,
                            rows,
                        } => {
                            if let Some((hosted, viewer)) = attached.take() {
                                hosted.detach(viewer);
                            }
                            match self.shell_for(&session) {
                                Ok((shell, row_id)) => {
                                    match self.info(&row_id) {
                                        Ok(session) => send(Response::Attached { session }),
                                        Err(error) => {
                                            send(*error);
                                            continue;
                                        }
                                    }
                                    let viewer = self.next_viewer.fetch_add(1, Ordering::SeqCst);
                                    shell.attach(Viewer {
                                        id: viewer,
                                        tx: tx.clone(),
                                        cols,
                                        rows,
                                    });
                                    attached = Some((shell, viewer));
                                }
                                Err(error) => send(*error),
                            }
                        }
                        Request::CloseShell { session } => send(
                            self.resolve(&session)
                                .map(|row| {
                                    self.end_shell(&row.id);
                                    Response::Done
                                })
                                .unwrap_or_else(|error| *error),
                        ),
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
                        Request::BringIn { session, term } => send(
                            self.bring_in(&session, term)
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
                        Request::RemoveConversation { session, way } => send(
                            self.remove_conversation(&session, way)
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
                        Request::SetGroupColor { group, color } => send(
                            self.set_group_color(group, color.as_deref())
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
                        Request::Focus {} => {
                            if let Some((hosted, viewer)) = &attached {
                                hosted.focus(*viewer);
                            }
                        }
                        Request::Pair { .. } => send(Response::error(
                            ErrorCode::BadRequest,
                            "pairing is for phones, through remote control",
                        )),
                        Request::Unpair {} => send(Response::error(
                            ErrorCode::BadRequest,
                            "unpairing is for phones, through remote control",
                        )),
                        Request::RemoteStatus {} => {
                            send(self.remote_status(self.remote_enabled_setting()));
                        }
                        Request::RemoteSet { enabled } => send(self.remote_set(enabled)),
                        Request::RemotePair {} => {
                            send(self.remote_pair(tx.clone(), connection));
                        }
                        Request::RemoteDevices {} => send(self.remote_devices()),
                        Request::RemoteRemove { device } => send(self.remote_remove(&device)),
                    }
                }
            }
        }

        if let Some((hosted, viewer)) = attached {
            hosted.detach(viewer);
        }
        // A pairing code this client showed stops working with it (add-remote-gateway D13).
        self.remote_client_left(connection);
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
        let starting = self.is_starting(&row.id);
        let start_error = self.start_error(&row.id);
        let offered = match row.origin {
            Origin::Hosted => true,
            Origin::History => row.agent_session_id.as_ref().is_some_and(|conversation| {
                lock(&self.offered).contains(&(row.agent, conversation.clone()))
            }),
        };
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
            starting,
            start_error,
            offered,
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
        let folder = canonical_folder(&row.cwd)?;
        let program = self.resolve_agent(row.agent)?;
        let row = self.follow_continuation(row);
        self.refuse_running_claude(&row, &program)?;
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
        let saved = conversation_saved(row.agent, row.agent_session_id.as_deref());
        let conversation = match row.agent_session_id.as_deref() {
            // Claude Code saves a conversation only once it has a message, and refuses to
            // resume one it never saved: start it afresh under the same id, as empty as
            // it was (add-inactive-sessions D7).
            Some(id) if !saved => Conversation::New { id },
            id => Conversation::Resume(id),
        };
        let spawned = self.spawn(SpawnPlan {
            id: &row.id,
            agent: row.agent,
            folder: &folder,
            program,
            conversation,
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
        self.clear_start_error(&row.id);
        self.log.line(&format!(
            "session started id={} agent={} resume=true known_conversation={} saved={saved}",
            row.id,
            row.agent,
            row.agent_session_id.is_some()
        ));
        self.broadcast(&row.id);
        self.info(&row.id)
    }

    /// The executable for `agent`: the settings file's pin, else discovery across the
    /// process `PATH`, the login-shell `PATH`, and the platform's install folders.
    /// Stops `reference`'s conversation in Claude Code's background, by the background id
    /// `claude stop` takes (not the conversation id, which it does not match), waits until
    /// Claude Code no longer lists it, then resumes it here (fix-claude-conversations D3).
    fn bring_in(self: &Arc<Self>, reference: &str, term: Option<String>) -> OpResult<SessionInfo> {
        self.refuse_while_stopping()?;
        let row = self.follow_continuation(self.resolve(reference)?);
        let claude = self.resolve_agent(row.agent)?;
        let Some(conversation) = row.agent_session_id.clone() else {
            return self.resume(reference, term);
        };
        *lock(&self.claude_running) = None;
        let running = external::claude_running(&claude)
            .map_err(|error| fail(ErrorCode::SpawnFailed, error))?;
        let background = running.iter().find(|session| {
            session.background && session.session_id.as_deref() == Some(conversation.as_str())
        });
        if let Some(job) = background.and_then(|session| session.background_id.clone()) {
            self.log
                .line(&format!("bringing in id={} from background", row.id));
            external::claude_stop(&claude, &job)
                .map_err(|error| fail(ErrorCode::SpawnFailed, error))?;
            let deadline = Instant::now() + BACKGROUND_STOP_WAIT;
            loop {
                let still = external::claude_running(&claude).is_ok_and(|running| {
                    running
                        .iter()
                        .any(|session| session.session_id.as_deref() == Some(conversation.as_str()))
                });
                if !still {
                    break;
                }
                if Instant::now() >= deadline {
                    return Err(fail(
                        ErrorCode::RunningInBackground,
                        format!(
                            "session `{}` is still running in Claude Code's background",
                            row.name
                        ),
                    ));
                }
                thread::sleep(Duration::from_millis(250));
            }
            lock(&self.outside).linked.remove(&row.id);
        }
        *lock(&self.claude_running) = None;
        self.resume(reference, term)
    }

    /// Claude Code's running conversations, asked at most every few seconds: a host start
    /// resumes several sessions in a row, and each ask costs about a second.
    fn claude_running(&self, claude: &Path) -> Result<Vec<external::ExternalSession>, String> {
        let mut cached = lock(&self.claude_running);
        if let Some((at, running)) = cached.as_ref()
            && at.elapsed() < CLAUDE_RUNNING_REUSE
        {
            return Ok(running.clone());
        }
        let running = external::claude_running(claude)?;
        *cached = Some((Instant::now(), running.clone()));
        Ok(running)
    }

    /// A Claude session that never saw its agent continue its conversation resumes the
    /// newest continuation instead, and keeps it (fix-claude-conversations D1).
    fn follow_continuation(&self, mut row: SessionRow) -> SessionRow {
        let (Agent::Claude, Some(stored)) = (row.agent, row.agent_session_id.as_deref()) else {
            return row;
        };
        let Some(projects) = claude_projects() else {
            return row;
        };
        let latest = hq_signals::history::claude_latest_conversation(&projects, stored);
        if latest == stored {
            return row;
        }
        self.record_conversation(row.agent, &row.id, &latest);
        self.log
            .line(&format!("conversation continued id={}", row.id));
        row.agent_session_id = Some(latest);
        row
    }

    /// Claude Code refuses to resume a conversation it is running already, in a terminal or
    /// in its own background daemon: say which, instead of starting an agent that exits at
    /// once (fix-claude-conversations D3). If Claude Code cannot be asked, the resume goes on.
    fn refuse_running_claude(&self, row: &SessionRow, claude: &Path) -> OpResult<()> {
        let (Agent::Claude, Some(conversation)) = (row.agent, row.agent_session_id.as_deref())
        else {
            return Ok(());
        };
        // Only a saved conversation can be running anywhere.
        if !conversation_saved(row.agent, Some(conversation)) {
            return Ok(());
        }
        let running = match self.claude_running(claude) {
            Ok(running) => running,
            Err(error) => {
                self.log
                    .line(&format!("claude agents unavailable before resume: {error}"));
                return Ok(());
            }
        };
        let Some(found) = running
            .iter()
            .find(|session| session.session_id.as_deref() == Some(conversation))
        else {
            return Ok(());
        };
        let pid = found
            .pid
            .map_or_else(|| "unknown".to_owned(), |pid| pid.to_string());
        if found.background {
            self.log.line(&format!(
                "resume refused id={} running in background",
                row.id
            ));
            return Err(fail(
                ErrorCode::RunningInBackground,
                format!(
                    "session `{}` is running in Claude Code's background (pid {pid})",
                    row.name
                ),
            ));
        }
        Err(fail(
            ErrorCode::ExternalLive,
            format!(
                "session `{}` is running in another terminal (pid {pid}); close it there first",
                row.name
            ),
        ))
    }

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
            self.login_path_settled.store(true, Ordering::SeqCst);
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
        if let Some(path) = &path {
            hq_discovery::shell::remember_login_path(path);
        }
        *lock(&self.login_path) = path;
        self.login_path_settled.store(true, Ordering::SeqCst);
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
        self.log
            .line(&format!("sidebar added ids={}", ids.join(",")));
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
        // Which rows leave the sidebar, so a row that vanishes can be traced
        // (fix-claude-conversations D5). Ids only.
        self.log
            .line(&format!("sidebar removed ids={}", ids.join(",")));
        for id in &ids {
            self.end_shell(id);
            self.settle_start(id);
            self.clear_start_error(id);
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
                color: row.color,
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
            color: None,
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

    /// Picks a group's colour from the palette, or `None` for automatic.
    fn set_group_color(&self, id: i64, color: Option<&str>) -> OpResult<()> {
        self.group_row(id)?;
        if let Some(name) = color
            && hq_protocol::GroupColor::from_name(name).is_none()
        {
            return Err(bad_request(format!("`{name}` is not a group colour")));
        }
        lock(&self.store)
            .set_group_color(id, color)
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
        command.env(LINKS_ENV.0, LINKS_ENV.1);
        command.env("PATH", hq_discovery::shell::agent_path(&plan.program));
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
            conversation: match plan.conversation {
                Conversation::New { id } | Conversation::Resume(Some(id)) => Some(id.to_owned()),
                Conversation::Resume(None) => None,
            },
            shell: false,
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
        // Closed with the ✕: the session leaves the sidebar. Any other end keeps it there,
        // inactive, and a host stop marked it to start again (add-inactive-sessions D2).
        // Its shell ends either way.
        self.end_shell(&hosted.id);
        if self.is_starting(&hosted.id) && !self.stopping.load(Ordering::SeqCst) {
            let code = exit_code.map_or_else(|| "unknown".to_owned(), |code| code.to_string());
            self.fail_start(
                &hosted.id,
                format!("the agent exited as it started (code {code})"),
            );
        }
        if hosted.closing()
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
        self.note_left_in_background(hosted);
        self.broadcast(&hosted.id);
    }

    /// Claude Code can keep a conversation running in its own background after the agent
    /// the host started ends; logged at once (ids only), so a later resume that is refused
    /// is not the first sign (fix-claude-conversations D3). Skipped while the host stops.
    fn note_left_in_background(&self, hosted: &Hosted) {
        if hosted.agent != Agent::Claude || self.stopping.load(Ordering::SeqCst) {
            return;
        }
        let (Some(conversation), Ok(claude)) =
            (hosted.conversation_id(), self.resolve_agent(Agent::Claude))
        else {
            return;
        };
        *lock(&self.claude_running) = None;
        let left = external::claude_running(&claude).is_ok_and(|running| {
            running.iter().any(|session| {
                session.background && session.session_id.as_deref() == Some(conversation.as_str())
            })
        });
        if left {
            self.log.line(&format!(
                "conversation still running in background id={}",
                hosted.id
            ));
        }
    }

    /// The session's shell, started first when none runs: the owner's login shell in
    /// the session's folder (add-session-shell-panel D1). Also returns the session's id.
    fn shell_for(self: &Arc<Self>, reference: &str) -> OpResult<(Arc<Hosted>, String)> {
        let row = self.resolve(reference)?;
        if let Some(shell) = lock(&self.shells).get(&row.id) {
            return Ok((Arc::clone(shell), row.id));
        }
        if !Path::new(&row.cwd).is_dir() {
            return Err(fail(
                ErrorCode::FolderNotFound,
                format!("the folder `{}` no longer exists", row.cwd),
            ));
        }
        let failed = |what: &str, error: &dyn std::fmt::Display| {
            fail(ErrorCode::SpawnFailed, format!("{what}: {error}"))
        };
        let program = std::env::var("SHELL")
            .ok()
            .filter(|shell| !shell.is_empty())
            .unwrap_or_else(|| "/bin/sh".to_owned());
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|error| failed("cannot open a pseudo-terminal", &error))?;
        let mut command = CommandBuilder::new(&program);
        command.arg("-l");
        command.cwd(&row.cwd);
        for marker in adapters::PARENT_SESSION_MARKERS {
            command.env_remove(marker);
        }
        command.env_remove(SESSION_ENV);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        command.env(LINKS_ENV.0, LINKS_ENV.1);
        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| failed(&format!("cannot start {program}"), &error))?;
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
        let shell = Arc::new(Hosted::new(HostedParts {
            id: format!("{}:shell", row.id),
            agent: row.agent,
            cwd: row.cwd.clone(),
            started_at_ms: now,
            writer,
            master: pair.master,
            killer: child.clone_killer(),
            pid: child.process_id(),
            tracker: SessionTracker::new(false, Thresholds::default(), now),
            conversation_known: false,
            conversation: None,
            shell: true,
        }));
        lock(&self.shells).insert(row.id.clone(), Arc::clone(&shell));
        self.log.line(&format!("shell started session={}", row.id));
        let pump = Arc::clone(&shell);
        thread::spawn(move || {
            let mut reader = reader;
            let mut buffer = [0u8; 16 * 1024];
            while let Ok(count) = reader.read(&mut buffer) {
                if count == 0 {
                    break;
                }
                pump.output(&buffer[..count]);
            }
        });
        let host = Arc::clone(self);
        let session = row.id.clone();
        let ended = Arc::clone(&shell);
        let mut child = child;
        thread::spawn(move || {
            let exit_code = child.wait().ok().map(|status| status.exit_code());
            {
                let mut shells = lock(&host.shells);
                if shells
                    .get(&session)
                    .is_some_and(|known| Arc::ptr_eq(known, &ended))
                {
                    shells.remove(&session);
                }
            }
            ended.end_viewers(exit_code);
            host.log.line(&format!("shell ended session={session}"));
        });
        Ok((shell, row.id))
    }

    /// Hangs up the session's shell, if one runs.
    fn end_shell(&self, session: &str) {
        let shell = lock(&self.shells).get(session).cloned();
        if let Some(shell) = shell {
            shell.kill();
        }
    }

    /// Persists and announces a status change, and looks up the agent's own
    /// conversation id after its first completed turn.
    fn refresh(&self, hosted: &Arc<Hosted>) {
        // A shell has no status to track (add-session-shell-panel D3).
        if hosted.shell {
            return;
        }
        if hosted.has_reported() {
            self.settle_start(&hosted.id);
        }
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
        if change.previous == Some(Status::Working)
            && matches!(status, Status::DoneUnread | Status::Done)
        {
            match hosted.agent {
                Agent::Codex => self.codex_turn_ended(),
                Agent::Agy => self.agy_turn_ended(),
                Agent::Claude => {}
            }
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
                self.record_conversation(hosted.agent, &hosted.id, &conversation);
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
        self.send_change(session);
    }

    /// Sends a session's state to every subscriber; for entries the index does not
    /// keep, such as outside agents with no known conversation.
    fn send_change(&self, session: SessionInfo) {
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
        if let Some(conversation) = record.session_id.as_deref() {
            let start = (record.event == EventKind::Started)
                .then(|| start_source(&payload.raw).unwrap_or_default());
            if !self.own_report(&hosted, conversation, start.as_deref(), now) {
                return;
            }
        }
        hosted.apply_hook(record.event, now);
        self.refresh(&hosted);
    }

    /// Whether a report from `conversation` is `hosted`'s own, recording a conversation it
    /// adopts and logging (ids only) one it ignores (fix-hook-session-match).
    pub(super) fn own_report(
        &self,
        hosted: &Hosted,
        conversation: &str,
        start: Option<&str>,
        now: u64,
    ) -> bool {
        match hosted.report_from(conversation, start) {
            Report::Own => true,
            Report::Adopted => {
                *lock(&hosted.conversation_known) = true;
                self.record_conversation(hosted.agent, &hosted.id, conversation);
                true
            }
            Report::Foreign if self.continued_in(hosted, conversation, now) => {
                hosted.continue_in(conversation);
                self.record_conversation(hosted.agent, &hosted.id, conversation);
                self.log
                    .line(&format!("conversation continued id={}", hosted.id));
                true
            }
            Report::Foreign => {
                if hosted.log_foreign_now(now) {
                    self.log
                        .line(&format!("hook from another conversation id={}", hosted.id));
                }
                false
            }
        }
    }

    /// Whether Claude Code carried `hosted`'s conversation on under `conversation`, read
    /// from the current conversation's transcript (fix-claude-conversations D1).
    fn continued_in(&self, hosted: &Hosted, conversation: &str, now: u64) -> bool {
        if hosted.agent != Agent::Claude || !hosted.check_continuation_now(now) {
            return false;
        }
        let (Some(current), Some(projects)) = (hosted.conversation_id(), claude_projects()) else {
            return false;
        };
        hq_signals::history::claude_continuation(&projects, &current).as_deref()
            == Some(conversation)
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
            // Remote control keeps the host up: a phone has no way to start it.
            if sessions.is_empty()
                && self.clients.load(Ordering::SeqCst) == 0
                && !self.remote.running()
            {
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
        let sessions: Vec<Arc<Hosted>> = lock(&self.live)
            .values()
            .chain(lock(&self.shells).values())
            .cloned()
            .collect();
        // What runs now starts again with the next host (add-inactive-sessions D2).
        let running: Vec<String> = sessions
            .iter()
            .filter(|hosted| !hosted.shell && !hosted.closing())
            .map(|hosted| hosted.id.clone())
            .collect();
        if let Err(error) = lock(&self.store).mark_for_start(&running, now_ms()) {
            self.log.line(&format!("index update failed error={error}"));
        }
        for hosted in &sessions {
            hosted.kill();
        }
        let deadline = Instant::now() + STOP_GRACE;
        while !(lock(&self.live).is_empty() && lock(&self.shells).is_empty())
            && Instant::now() < deadline
        {
            thread::sleep(Duration::from_millis(50));
        }
        if let Err(error) = lock(&self.store).end_all_live(now_ms()) {
            self.log.line(&format!("index update failed error={error}"));
        }
        // Phones hear the host go at once instead of waiting out QUIC's idle timeout.
        self.stop_gateway();
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

/// What started a Claude Code session, from a `SessionStart` hook: `startup`, `resume`,
/// `clear` or `compact`.
fn start_source(raw: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    value.get("source")?.as_str().map(str::to_owned)
}
