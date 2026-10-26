//! The phone's core (add-android-app D1): pairing, the computers it knows, the connection
//! to the computer on screen, home as the desktop arranges it, and open terminals. The
//! Android app draws and handles the phone's lifecycle. Calls block, so the app makes
//! them off its main thread. The app reads `version()` each frame and takes a new
//! snapshot when it moves. Nothing calls back into Kotlin.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use hq_protocol::{AgentUsage, Frame, GroupInfo, Request, Response, SessionInfo};
use hq_remote::PairingCode;
use hq_signals::status::Status;
use iroh::SecretKey;
use iroh::endpoint::Connection;
use tokio::sync::oneshot;

mod computers;
mod link;
mod mirror;
mod terminal;

use computers::{Computer, Computers};
use link::{Command, Link};
pub use mirror::{GroupRows, Home, PastConversation, Row, RowStatus, UsageMeter};
use terminal::Terminal;
pub use terminal::{Key, Paint, Screen, Span};

uniffi::setup_scaffolding!();

/// How long a request may wait for the host's answer.
const ANSWER_WAIT: Duration = Duration::from_secs(15);
/// Typed text and its Enter go separately, or agents read them as a paste (spike).
const ENTER_DELAY: Duration = Duration::from_millis(80);

#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum MobileError {
    /// Something failed. `reason` is for the owner.
    #[error("{reason}")]
    Failed { reason: String },
    /// The computer refused, such as a pairing code that no longer works.
    #[error("{reason}")]
    Refused { reason: String },
    #[error("not connected to the computer")]
    NotConnected,
}

fn failed(reason: impl ToString) -> MobileError {
    MobileError::Failed {
        reason: reason.to_string(),
    }
}

/// Where the connection to the computer on screen stands.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum LinkStatus {
    /// No computer paired yet.
    NoComputer,
    /// The app is away. Nothing is connected.
    Off,
    Connecting,
    Connected,
    /// Lost. Retrying since `since_ms` (Unix milliseconds).
    Reconnecting {
        since_ms: u64,
        attempts: u32,
    },
    /// The computer refused this phone: it was removed there.
    NotKnown,
    /// The computer's Headquarter is older than this app speaks.
    ComputerTooOld {
        version: String,
    },
    /// The computer's Headquarter is newer than this app speaks.
    AppTooOld {
        version: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct ComputerInfo {
    pub id: String,
    pub name: String,
    pub relay: Option<String>,
    pub paired_at: u64,
    pub last_seen_at: Option<u64>,
    pub selected: bool,
}

/// What a response from the host was.
pub(crate) enum Taken {
    Change,
    /// A change that makes the open session read: the link tells the host.
    Read(String),
    /// The answer to the oldest request.
    Answer(Box<Response>),
}

/// What a terminal on the phone shows.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Open {
    session: String,
    shell: bool,
    cols: u16,
    rows: u16,
}

struct State {
    computers: Computers,
    link: LinkStatus,
    mirror: mirror::Mirror,
    /// When the mirror was last known to be current (Unix milliseconds).
    current_at: Option<u64>,
    connection: Option<Connection>,
    open: Option<Open>,
    terminal: Option<Terminal>,
    /// The open session's agent exited.
    ended: bool,
    /// The owner looks at the open terminal: replies arriving there are read.
    watching: bool,
}

pub(crate) struct Shared {
    folder: PathBuf,
    version: AtomicU64,
    /// Wakes `wait_change` when `version` moves. The mutex guards nothing else.
    moved: (Mutex<()>, Condvar),
    state: Mutex<State>,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn changed(&self) {
        // Moving the counter under the lock means a waiter cannot miss it between its
        // check and its wait.
        let _moving = self
            .moved
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        self.version.fetch_add(1, Ordering::SeqCst);
        self.moved.1.notify_all();
    }

    pub(crate) fn computer(&self, id: &str) -> Option<Computer> {
        self.lock()
            .computers
            .list
            .iter()
            .find(|known| known.id == id)
            .cloned()
    }

    pub(crate) fn set_link(&self, status: LinkStatus) {
        self.lock().link = status;
        self.changed();
    }

    /// The gateway's welcome: the computer's current name and relay (D4).
    pub(crate) fn welcomed(&self, id: &str, name: String, relay: Option<String>) {
        let mut state = self.lock();
        if let Some(computer) = state.computers.get_mut(id) {
            computer.name = name;
            if relay.is_some() {
                computer.relay = relay;
            }
            computer.last_seen_at = Some(hq_signals::now_ms());
        }
        // A failed save keeps the old relay for next time. This connection goes on.
        let _ = state.computers.save(&self.folder);
        drop(state);
        self.changed();
    }

    pub(crate) fn listed(&self, sessions: Vec<SessionInfo>, groups: Vec<GroupInfo>) {
        let mut state = self.lock();
        state.mirror.sessions = sessions;
        state.mirror.groups = groups;
        drop(state);
        self.changed();
    }

    pub(crate) fn usage(&self, agents: Vec<AgentUsage>) {
        self.lock().mirror.usage = agents;
        self.changed();
    }

    pub(crate) fn connected(&self, connection: Connection) {
        let mut state = self.lock();
        state.connection = Some(connection);
        state.link = LinkStatus::Connected;
        state.current_at = Some(hq_signals::now_ms());
        drop(state);
        self.changed();
    }

    /// `connection` ended. A link stopping while a newer one connects leaves the newer alone.
    pub(crate) fn disconnected(&self, connection: &Connection) {
        let mut state = self.lock();
        let current = state
            .connection
            .as_ref()
            .is_some_and(|known| known.stable_id() == connection.stable_id());
        if !current {
            return;
        }
        state.connection = None;
        if state.link == LinkStatus::Connected {
            state.current_at = Some(hq_signals::now_ms());
        }
        drop(state);
        self.changed();
    }

    /// The terminal to reattach after a reconnect, started afresh: the host replays its
    /// catch-up.
    pub(crate) fn reopen(&self) -> Option<Open> {
        let mut state = self.lock();
        let open = state.open.clone()?;
        state.terminal = Some(Terminal::new(open.cols, open.rows));
        state.ended = false;
        Some(open)
    }

    /// Takes in a response from the host. Changes update the mirror. Anything else is the
    /// answer to the oldest request.
    pub(crate) fn take_in(&self, response: Response) -> Taken {
        let mut state = self.lock();
        match response {
            Response::SessionChanged { session } => {
                // A reply that arrives while the owner watches it is read right away, as
                // on the desktop.
                let read = state.watching
                    && session.status == Status::DoneUnread
                    && state
                        .open
                        .as_ref()
                        .is_some_and(|open| open.session == session.id && !open.shell);
                let id = session.id.clone();
                state.mirror.session_changed(session);
                drop(state);
                self.changed();
                match read {
                    true => Taken::Read(id),
                    false => Taken::Change,
                }
            }
            Response::GroupsChanged { groups } => {
                state.mirror.groups = groups;
                drop(state);
                self.changed();
                Taken::Change
            }
            Response::UsageChanged { agents } => {
                state.mirror.usage = agents;
                drop(state);
                self.changed();
                Taken::Change
            }
            Response::DetailsChanged { .. } | Response::RemoteWelcome { .. } => Taken::Change,
            Response::Ended { session, .. } => {
                if state
                    .open
                    .as_ref()
                    .is_some_and(|open| open.session == session)
                {
                    state.ended = true;
                }
                drop(state);
                self.changed();
                Taken::Change
            }
            answer => Taken::Answer(Box::new(answer)),
        }
    }

    /// Feeds output to the open terminal. Returns its replies.
    pub(crate) fn output(&self, bytes: &[u8]) -> Vec<u8> {
        let replies = match self.lock().terminal.as_mut() {
            Some(terminal) => terminal.feed(bytes),
            None => Vec::new(),
        };
        self.changed();
        replies
    }
}

/// A new key for this phone, made once on first launch. The app keeps it encrypted by
/// the Android Keystore and gives it to [`Phone::new`] on each start (D2).
#[uniffi::export]
pub fn new_phone_key() -> Vec<u8> {
    SecretKey::generate().to_bytes().to_vec()
}

#[derive(uniffi::Object)]
pub struct Phone {
    runtime: tokio::runtime::Runtime,
    shared: Arc<Shared>,
    key: SecretKey,
    link: Mutex<Option<Link>>,
}

impl Phone {
    fn link_tx(&self) -> Result<tokio::sync::mpsc::UnboundedSender<Command>, MobileError> {
        self.link
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .as_ref()
            .map(|link| link.tx.clone())
            .ok_or(MobileError::NotConnected)
    }

    /// Sends a request and waits for the host's answer. An error answer is an `Err`.
    fn ask(&self, request: Request) -> Result<Response, MobileError> {
        let tx = self.link_tx()?;
        let (answer, wait) = oneshot::channel();
        tx.send(Command::Ask { request, answer })
            .map_err(|_| MobileError::NotConnected)?;
        let response = self.runtime.block_on(async {
            tokio::time::timeout(ANSWER_WAIT, wait)
                .await
                .map_err(|_| failed("the computer did not answer in time"))?
                .map_err(|_| MobileError::NotConnected)
        })?;
        match response {
            Response::Error { message, .. } => Err(failed(message)),
            response => Ok(response),
        }
    }

    fn tell(&self, command: Command) {
        if let Ok(tx) = self.link_tx() {
            let _ = tx.send(command);
        }
    }

    fn open_terminal(&self, open: Open) -> Result<(), MobileError> {
        let request = match open.shell {
            true => Request::AttachShell {
                session: open.session.clone(),
                cols: open.cols,
                rows: open.rows,
            },
            false => Request::Attach {
                session: open.session.clone(),
                cols: open.cols,
                rows: open.rows,
            },
        };
        {
            let mut state = self.shared.lock();
            state.terminal = Some(Terminal::new(open.cols, open.rows));
            state.open = Some(open.clone());
            state.ended = false;
            state.watching = true;
        }
        if let Err(error) = self.ask(request) {
            let mut state = self.shared.lock();
            state.terminal = None;
            state.open = None;
            return Err(error);
        }
        let unread = !open.shell
            && self
                .shared
                .lock()
                .mirror
                .session(&open.session)
                .is_some_and(|session| session.status == Status::DoneUnread);
        if unread {
            let _ = self.ask(Request::Seen {
                session: open.session,
            });
        }
        self.shared.changed();
        Ok(())
    }
}

#[uniffi::export]
impl Phone {
    /// The phone's core with its key and the app's private folder.
    #[uniffi::constructor]
    pub fn new(key: Vec<u8>, folder: String) -> Result<Arc<Phone>, MobileError> {
        let bytes: [u8; 32] = key
            .try_into()
            .map_err(|_| failed("the phone's key is damaged"))?;
        let folder = PathBuf::from(folder);
        let computers = Computers::load(&folder).map_err(failed)?;
        let link = match computers.selected() {
            Some(_) => LinkStatus::Off,
            None => LinkStatus::NoComputer,
        };
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .thread_name("hq-mobile")
            .build()
            .map_err(failed)?;
        Ok(Arc::new(Phone {
            runtime,
            shared: Arc::new(Shared {
                folder,
                version: AtomicU64::new(0),
                moved: (Mutex::new(()), Condvar::new()),
                state: Mutex::new(State {
                    computers,
                    link,
                    mirror: mirror::Mirror::default(),
                    current_at: None,
                    connection: None,
                    open: None,
                    terminal: None,
                    ended: false,
                    watching: false,
                }),
            }),
            key: SecretKey::from_bytes(&bytes),
            link: Mutex::new(None),
        }))
    }

    /// Moves whenever anything below changes.
    pub fn version(&self) -> u64 {
        self.shared.version.load(Ordering::SeqCst)
    }

    /// Blocks until the version moves past `seen`, or for `timeout_ms`, and returns it. The
    /// app waits here on a background thread instead of reading `version` every frame.
    pub fn wait_change(&self, seen: u64, timeout_ms: u32) -> u64 {
        let deadline = Instant::now() + Duration::from_millis(timeout_ms.into());
        let (lock, moved) = &self.shared.moved;
        let mut guard = lock
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        loop {
            let now = self.shared.version.load(Ordering::SeqCst);
            let left = deadline.saturating_duration_since(Instant::now());
            if now != seen || left.is_zero() {
                return now;
            }
            guard = moved
                .wait_timeout(guard, left)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
        }
    }

    pub fn computers(&self) -> Vec<ComputerInfo> {
        let state = self.shared.lock();
        state
            .computers
            .list
            .iter()
            .map(|computer| ComputerInfo {
                id: computer.id.clone(),
                name: computer.name.clone(),
                relay: computer.relay.clone(),
                paired_at: computer.paired_at,
                last_seen_at: computer.last_seen_at,
                selected: state.computers.selected.as_deref() == Some(computer.id.as_str()),
            })
            .collect()
    }

    pub fn status(&self) -> LinkStatus {
        self.shared.lock().link.clone()
    }

    /// When home was last known to be current, while reconnecting.
    pub fn current_at(&self) -> Option<u64> {
        self.shared.lock().current_at
    }

    pub fn home(&self) -> Home {
        self.shared.lock().mirror.home(hq_signals::now_ms())
    }

    pub fn past_conversations(&self) -> Vec<PastConversation> {
        self.shared.lock().mirror.past(hq_signals::now_ms())
    }

    /// How the connection runs: "direct" or "relayed", with the round trip.
    pub fn path(&self) -> Option<String> {
        let connection = self.shared.lock().connection.clone()?;
        let paths = connection.paths();
        let selected = paths.iter().find(|path| path.is_selected())?;
        let kind = if selected.is_relay() {
            "relayed"
        } else {
            "direct"
        };
        Some(format!("{kind} · {} ms", selected.rtt().as_millis()))
    }

    /// Pairs with the computer showing `code`, names this phone `name`, selects the
    /// computer and connects to it (D3).
    pub fn pair(&self, code: String, name: String) -> Result<ComputerInfo, MobileError> {
        let code = PairingCode::parse(&code).map_err(|_| MobileError::Refused {
            reason: "This isn't a Headquarter pairing code.".into(),
        })?;
        self.disconnect();
        let (host, relay) = self.runtime.block_on(pair_once(&self.key, &code, name))?;
        let now = hq_signals::now_ms();
        let computer = Computer {
            id: code.host.to_string(),
            name: host,
            relay: relay.or_else(|| code.relay.as_ref().map(ToString::to_string)),
            addrs: code.addrs.iter().map(ToString::to_string).collect(),
            paired_at: now,
            last_seen_at: Some(now),
        };
        {
            let mut state = self.shared.lock();
            state.computers.upsert(computer.clone());
            state.computers.save(&self.shared.folder).map_err(failed)?;
            state.mirror = mirror::Mirror::default();
            state.link = LinkStatus::Off;
        }
        self.shared.changed();
        self.connect();
        Ok(ComputerInfo {
            id: computer.id,
            name: computer.name,
            relay: computer.relay,
            paired_at: computer.paired_at,
            last_seen_at: computer.last_seen_at,
            selected: true,
        })
    }

    /// Shows another computer: disconnects from the current one and connects to it.
    pub fn select(&self, id: String) -> Result<(), MobileError> {
        self.disconnect();
        {
            let mut state = self.shared.lock();
            if !state.computers.select(&id) {
                return Err(failed("this computer is not paired"));
            }
            state.computers.save(&self.shared.folder).map_err(failed)?;
            state.mirror = mirror::Mirror::default();
            state.current_at = None;
        }
        self.shared.changed();
        self.connect();
        Ok(())
    }

    /// Connects to the selected computer, unless already connected to it (the app came
    /// to the screen).
    pub fn connect(&self) {
        let Some(id) = self.shared.lock().computers.selected.clone() else {
            self.shared.set_link(LinkStatus::NoComputer);
            return;
        };
        let mut link = self
            .link
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let status = self.shared.lock().link.clone();
        // A link that gave up (versions differ, phone unknown) tries again when the app
        // returns: the owner may have updated the computer meanwhile.
        let gave_up = matches!(
            status,
            LinkStatus::ComputerTooOld { .. } | LinkStatus::AppTooOld { .. } | LinkStatus::NotKnown
        );
        if !gave_up && link.as_ref().is_some_and(|link| link.computer == id) {
            return;
        }
        let connected = status == LinkStatus::Connected;
        let started = Link::start(
            &self.runtime,
            Arc::clone(&self.shared),
            self.key.clone(),
            id,
        );
        if let Some(previous) = link.replace(started) {
            previous.stop(&self.runtime, connected);
        }
    }

    /// Disconnects (the app left the screen). Home keeps what it knew.
    pub fn disconnect(&self) {
        let link = self
            .link
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        let connected = self.shared.lock().link == LinkStatus::Connected;
        if let Some(link) = link {
            link.stop(&self.runtime, connected);
        }
        let mut state = self.shared.lock();
        state.connection = None;
        state.link = match state.computers.selected() {
            Some(_) => LinkStatus::Off,
            None => LinkStatus::NoComputer,
        };
        drop(state);
        self.shared.changed();
    }

    /// Unpairs a computer (D5): removes this phone there when it is connected, then
    /// forgets it here. Returns whether the computer removed the phone.
    pub fn unpair(&self, id: String) -> Result<bool, MobileError> {
        let connected = {
            let state = self.shared.lock();
            state.computers.selected.as_deref() == Some(id.as_str())
                && state.link == LinkStatus::Connected
        };
        let removed = connected && matches!(self.ask(Request::Unpair {}), Ok(Response::Unpaired));
        self.disconnect();
        {
            let mut state = self.shared.lock();
            state.computers.remove(&id);
            state.computers.save(&self.shared.folder).map_err(failed)?;
            state.mirror = mirror::Mirror::default();
            state.current_at = None;
            state.link = match state.computers.selected() {
                Some(_) => LinkStatus::Off,
                None => LinkStatus::NoComputer,
            };
        }
        self.shared.changed();
        Ok(removed)
    }

    /// Opens a running session at `cols` × `rows`. The phone becomes the device in use.
    pub fn open_session(&self, id: String, cols: u16, rows: u16) -> Result<(), MobileError> {
        self.open_terminal(Open {
            session: id,
            shell: false,
            cols,
            rows,
        })
    }

    /// Opens a running session's shell, starting it on the computer when none runs.
    pub fn open_shell(&self, id: String, cols: u16, rows: u16) -> Result<(), MobileError> {
        self.open_terminal(Open {
            session: id,
            shell: true,
            cols,
            rows,
        })
    }

    /// Starts an inactive session again on the computer, on its conversation. The app
    /// then opens it at its own grid.
    pub fn resume(&self, id: String) -> Result<(), MobileError> {
        self.ask(Request::Resume {
            session: id,
            term: Some("xterm-256color".into()),
        })
        .map(|_| ())
    }

    /// Puts a past conversation in the sidebar, on the phone and the desktop.
    pub fn add(&self, id: String) -> Result<(), MobileError> {
        self.ask(Request::Add { sessions: vec![id] }).map(|_| ())
    }

    pub fn close_terminal(&self) {
        let was_open = {
            let mut state = self.shared.lock();
            state.terminal = None;
            state.ended = false;
            state.open.take().is_some()
        };
        if was_open {
            let _ = self.ask(Request::Detach {});
        }
        self.shared.changed();
    }

    pub fn screen(&self) -> Option<Screen> {
        self.shared.lock().terminal.as_ref().map(Terminal::screen)
    }

    /// The open session's agent exited.
    pub fn terminal_ended(&self) -> bool {
        self.shared.lock().ended
    }

    /// The owner looks at the open terminal, or left it for another screen.
    pub fn set_watching(&self, watching: bool) {
        self.shared.lock().watching = watching;
    }

    /// A new grid for the open terminal, such as after a pinch.
    pub fn resize(&self, cols: u16, rows: u16) {
        let resized = {
            let mut state = self.shared.lock();
            let Some(terminal) = state.terminal.as_mut() else {
                return;
            };
            if terminal.size() == (cols, rows) {
                false
            } else {
                terminal.resize(cols, rows);
                if let Some(open) = state.open.as_mut() {
                    (open.cols, open.rows) = (cols, rows);
                }
                true
            }
        };
        if resized {
            self.tell(Command::Tell(Request::Resize { cols, rows }));
            self.shared.changed();
        }
    }

    /// Scrolls the open terminal by `lines`. Positive goes back into history.
    pub fn scroll(&self, lines: i32) {
        if let Some(terminal) = self.shared.lock().terminal.as_mut() {
            terminal.scroll(lines);
        }
        self.shared.changed();
    }

    /// Types `text`, then Enter a moment later (D8).
    pub fn type_text(&self, text: String) -> Result<(), MobileError> {
        let tx = self.link_tx()?;
        self.follow_output();
        if !text.is_empty() {
            tx.send(Command::Input(text.into_bytes()))
                .map_err(|_| MobileError::NotConnected)?;
        }
        self.runtime.spawn(async move {
            tokio::time::sleep(ENTER_DELAY).await;
            let _ = tx.send(Command::Input(b"\r".to_vec()));
        });
        Ok(())
    }

    pub fn send_key(&self, key: Key) -> Result<(), MobileError> {
        let bytes = match self.shared.lock().terminal.as_ref() {
            Some(terminal) => terminal.key_bytes(key).to_vec(),
            None => return Err(failed("no terminal is open")),
        };
        self.follow_output();
        self.link_tx()?
            .send(Command::Input(bytes))
            .map_err(|_| MobileError::NotConnected)
    }

    /// The owner came back to the open terminal: it takes the phone's size again.
    pub fn focus(&self) {
        self.tell(Command::Tell(Request::Focus {}));
    }

    /// Home is on screen or not: the host reads usage only while a client shows it.
    pub fn set_home_visible(&self, visible: bool) {
        if let Ok(tx) = self.link_tx() {
            let (answer, _ignored) = oneshot::channel();
            let _ = tx.send(Command::Ask {
                request: Request::Visible { visible },
                answer,
            });
        }
    }
}

impl Phone {
    /// Typing returns the view to the latest output.
    fn follow_output(&self) {
        if let Some(terminal) = self.shared.lock().terminal.as_mut() {
            terminal.scroll_to_bottom();
        }
    }
}

/// Pairs over a connection of its own: `Pair`, then `Paired` and the welcome. Returns the
/// computer's name and relay.
async fn pair_once(
    key: &SecretKey,
    code: &PairingCode,
    name: String,
) -> Result<(String, Option<String>), MobileError> {
    let relays = link::relays_for(code.relay.as_ref().map(ToString::to_string).as_deref());
    let (endpoint, connection, mut send, mut recv) = link::dial(key, &relays, code.addr())
        .await
        .map_err(|reason| MobileError::Failed {
            reason: format!("Couldn't reach the computer: {reason}"),
        })?;
    let pair = Request::Pair {
        secret: code.secret.clone(),
        name,
    };
    hq_remote::write_frame(&mut send, &Frame::request(&pair))
        .await
        .map_err(failed)?;
    let mut paired = false;
    let outcome = loop {
        let frame = tokio::time::timeout(ANSWER_WAIT, hq_remote::read_frame(&mut recv))
            .await
            .map_err(|_| failed("the computer did not answer in time"))?;
        let Ok(Some(Frame::Control(bytes))) = frame else {
            break Err(failed("the computer closed the connection"));
        };
        match Response::parse(&bytes).map_err(failed)? {
            Response::Paired { .. } => paired = true,
            Response::RemoteWelcome { host, relay, .. } if paired => break Ok((host, relay)),
            Response::Error { message, .. } => {
                break Err(MobileError::Refused { reason: message });
            }
            _ => continue,
        }
    };
    connection.close(0u32.into(), b"paired");
    endpoint.close().await;
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_returns_when_the_version_moves_or_times_out() {
        let folder = std::env::temp_dir().join(format!("hq-mobile-wait-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let phone = Phone::new(new_phone_key(), folder.to_string_lossy().into_owned()).unwrap();
        let seen = phone.version();

        let started = Instant::now();
        assert_eq!(phone.wait_change(seen, 50), seen);
        assert!(started.elapsed() >= Duration::from_millis(50));

        let shared = Arc::clone(&phone.shared);
        let mover = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(30));
            shared.changed();
        });
        let started = Instant::now();
        assert_eq!(phone.wait_change(seen, 5_000), seen + 1);
        assert!(started.elapsed() < Duration::from_secs(2));
        mover.join().unwrap();
        // Already moved: no wait at all.
        assert_eq!(phone.wait_change(seen, 5_000), seen + 1);
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn the_welcome_keeps_the_computer_current() {
        let folder = std::env::temp_dir().join(format!("hq-mobile-welcome-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let mut computers = Computers::default();
        computers.upsert(Computer {
            id: "host".into(),
            name: "Old name".into(),
            relay: Some("https://old.example.com./".into()),
            addrs: Vec::new(),
            paired_at: 1,
            last_seen_at: None,
        });
        let phone = Phone::new(new_phone_key(), folder.to_string_lossy().into_owned()).unwrap();
        phone.shared.lock().computers = computers;
        phone.shared.welcomed(
            "host",
            "MacBook".into(),
            Some("https://new.example.com./".into()),
        );
        // No relay in a welcome keeps the one known.
        phone.shared.welcomed("host", "MacBook".into(), None);
        let saved = Computers::load(&folder).unwrap();
        let computer = saved.selected().unwrap();
        assert_eq!(computer.name, "MacBook");
        assert_eq!(computer.relay.as_deref(), Some("https://new.example.com./"));
        assert!(computer.last_seen_at.is_some());
        std::fs::remove_dir_all(folder).unwrap();
    }
}
