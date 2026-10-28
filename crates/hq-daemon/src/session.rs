//! One hosted agent session: its PTY, catch-up buffer, attached viewers, and status.
//! Terminal bytes live only in memory here and are never written to disk or the log.

use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use hq_protocol::{Frame, Holder, Response};
use hq_signals::event::{Agent, EventKind, NeedsReason};
use hq_signals::helpers::is_terminal_report;
use hq_signals::status::{SessionTracker, Status};
use portable_pty::{ChildKiller, MasterPty, PtySize};

use crate::modes::Modes;

/// Output kept for clients that attach later.
pub const CATCH_UP_BYTES: usize = 1024 * 1024;
const CATCH_UP_CHUNK: usize = 64 * 1024;
/// The longest a redraw keeps the agent's terminal one row short (fix-reattach-redraw).
const REDRAW_WAIT: Duration = Duration::from_millis(500);
/// Erase the screen: how agents start the full repaint a size change asks for.
const ERASE_SCREEN: &[u8] = b"\x1b[2J";

/// A redraw in progress (fix-reattach-redraw): the size to restore once the agent has
/// repainted at one row fewer. Any later size change replaces it (`generation`).
#[derive(Default)]
struct Redraw {
    generation: u64,
    restore: Option<(u16, u16)>,
    repainted: bool,
}

const FOREIGN_LOG_EVERY_MS: u64 = 60_000;
const CONTINUATION_CHECK_EVERY_MS: u64 = 5_000;

/// The conversation a Claude Code session reports for, and whether its agent has said so
/// itself yet with a `SessionStart`.
struct Conversation {
    id: Option<String>,
    started: bool,
}

/// Whose a status report is (`Hosted::report_from`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Report {
    Own,
    /// This session's conversation is now the report's.
    Adopted,
    /// Another conversation's, carrying this session's launch environment.
    Foreign,
}

/// What a connection's writer thread sends to its client.
pub enum Outgoing {
    Frame(Frame),
    Close,
}

pub struct Viewer {
    pub id: u64,
    pub tx: Sender<Outgoing>,
    pub cols: u16,
    pub rows: u16,
    /// The paired phone this viewer is on, named by the remote gateway; `None` for a
    /// client on the computer (fix-terminal-in-use-elsewhere D2).
    pub device: Option<String>,
}

/// Tells every viewer which of them the terminal is sized for, each from its own side
/// (fix-terminal-in-use-elsewhere D1).
fn announce_in_use(viewers: &[Viewer], active: Option<u64>) {
    let Some(holder) = active.and_then(|id| viewers.iter().find(|viewer| viewer.id == id)) else {
        return;
    };
    for viewer in viewers {
        let seen = if viewer.id == holder.id {
            Holder::Here
        } else {
            match &holder.device {
                Some(name) => Holder::Phone { name: name.clone() },
                None => Holder::Computer,
            }
        };
        let _ = viewer
            .tx
            .send(Outgoing::Frame(Frame::response(&Response::InUse {
                holder: seen,
            })));
    }
}

/// Which viewer is in use, and the order viewers were used in (add-remote-gateway D6).
#[derive(Default)]
struct InUse {
    active: Option<u64>,
    /// Viewer ids, most recently used last.
    order: Vec<u64>,
}

impl InUse {
    fn use_viewer(&mut self, id: u64) {
        self.order.retain(|known| *known != id);
        self.order.push(id);
        self.active = Some(id);
    }
}

pub struct Hosted {
    pub id: String,
    pub agent: Agent,
    pub cwd: String,
    pub started_at_ms: u64,
    /// The agent process id, so outside-session scans can leave it out.
    pub pid: Option<u32>,
    /// A session's shell rather than its agent: no status, index row or scans
    /// (add-session-shell-panel).
    pub shell: bool,
    writer: Mutex<Box<dyn Write + Send>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
    buffer: Mutex<CatchUp>,
    viewers: Mutex<Vec<Viewer>>,
    /// Lock order: viewers, then in_use.
    in_use: Mutex<InUse>,
    redraw: Mutex<Redraw>,
    repainted: Condvar,
    tracker: Mutex<SessionTracker>,
    reported: Mutex<Option<(Status, Option<NeedsReason>)>>,
    /// Set once a close has started, so a second request does nothing and the session
    /// leaves the sidebar whatever the agent's exit code.
    closing: AtomicBool,
    /// Whether the agent has reported its status itself (OSC 7501).
    program_status: AtomicBool,
    /// Whether the agent has reported anything itself, through OSC 7501 or a hook.
    has_spoken: AtomicBool,
    /// Whether the agent's own conversation id is known.
    pub conversation_known: Mutex<bool>,
    /// The Claude Code conversation this session reports for (fix-hook-session-match).
    /// Claude Code's own daemon can run other conversations with this session's launch
    /// environment; their reports are not this session's.
    conversation: Mutex<Conversation>,
    /// When a report from another conversation was last logged.
    foreign_logged_ms: Mutex<Option<u64>>,
    /// When the transcript was last read for a continuation (fix-claude-conversations D1).
    continuation_checked_ms: Mutex<Option<u64>>,
}

/// The latest output for clients that attach later, and the terminal modes in effect
/// where it starts.
struct CatchUp {
    bytes: VecDeque<u8>,
    modes: Modes,
}

/// A poisoned lock only means another thread panicked mid-update; the data is
/// still the best state available, so keep serving.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub struct HostedParts {
    pub id: String,
    pub agent: Agent,
    pub cwd: String,
    pub started_at_ms: u64,
    pub writer: Box<dyn Write + Send>,
    pub master: Box<dyn MasterPty + Send>,
    pub killer: Box<dyn ChildKiller + Send + Sync>,
    pub pid: Option<u32>,
    pub tracker: SessionTracker,
    pub conversation_known: bool,
    /// The Claude Code conversation the agent was launched on, when the launch names it.
    pub conversation: Option<String>,
    pub shell: bool,
}

pub struct StatusChange {
    pub status: Status,
    pub reason: Option<NeedsReason>,
    pub previous: Option<Status>,
    /// Whether the owner had prompted or typed since the agent started.
    pub engaged: bool,
}

impl StatusChange {
    /// Whether the agent did something. Reading a finished session, going idle, and an
    /// agent starting or resuming before the owner has typed (Codex loading a
    /// conversation, agy signing in) change the status too, but a row's age counts from
    /// the agent's last real change.
    pub fn is_activity(&self) -> bool {
        self.engaged
            && !matches!(
                (self.previous, self.status),
                (Some(Status::DoneUnread), Status::Done) | (_, Status::Inactive)
            )
    }
}

impl Hosted {
    pub fn new(parts: HostedParts) -> Hosted {
        Hosted {
            id: parts.id,
            agent: parts.agent,
            cwd: parts.cwd,
            started_at_ms: parts.started_at_ms,
            pid: parts.pid,
            shell: parts.shell,
            writer: Mutex::new(parts.writer),
            master: Mutex::new(parts.master),
            killer: Mutex::new(parts.killer),
            buffer: Mutex::new(CatchUp {
                bytes: VecDeque::new(),
                modes: Modes::new(),
            }),
            viewers: Mutex::new(Vec::new()),
            in_use: Mutex::new(InUse::default()),
            redraw: Mutex::new(Redraw::default()),
            repainted: Condvar::new(),
            tracker: Mutex::new(parts.tracker),
            reported: Mutex::new(None),
            closing: AtomicBool::new(false),
            program_status: AtomicBool::new(false),
            has_spoken: AtomicBool::new(false),
            conversation_known: Mutex::new(parts.conversation_known),
            conversation: Mutex::new(Conversation {
                id: parts.conversation,
                started: false,
            }),
            foreign_logged_ms: Mutex::new(None),
            continuation_checked_ms: Mutex::new(None),
        }
    }

    /// Whether a report from Claude Code `conversation` belongs to this session. `start`
    /// is the source of a `SessionStart` report. The agent's own first start names its
    /// conversation, which a resume can change, and `/clear` or `/compact` can change it
    /// again in place. Until then the launch's conversation, or the first one heard of when
    /// the launch named none, is the session's. Other agents' reports are always its own.
    pub fn report_from(&self, conversation: &str, start: Option<&str>) -> Report {
        if self.agent != Agent::Claude {
            return Report::Own;
        }
        let mut known = lock(&self.conversation);
        let restarted = match start {
            Some("clear" | "compact") => true,
            Some(_) => !known.started,
            None => false,
        };
        if start.is_some() && restarted {
            known.started = true;
        }
        match known.id.as_deref() {
            Some(id) if id == conversation => Report::Own,
            Some(_) if !restarted => Report::Foreign,
            _ => {
                known.id = Some(conversation.to_owned());
                Report::Adopted
            }
        }
    }

    /// The Claude Code conversation this session reports for, when known.
    pub fn conversation_id(&self) -> Option<String> {
        lock(&self.conversation).id.clone()
    }

    /// Claude Code carried this session's conversation on under `conversation`.
    pub fn continue_in(&self, conversation: &str) {
        lock(&self.conversation).id = Some(conversation.to_owned());
    }

    /// Whether to read the transcript for a continuation now: at most every few seconds,
    /// since a chatty other conversation would otherwise read it on every report.
    pub fn check_continuation_now(&self, now_ms: u64) -> bool {
        let mut last = lock(&self.continuation_checked_ms);
        if last.is_some_and(|at| now_ms.saturating_sub(at) < CONTINUATION_CHECK_EVERY_MS) {
            return false;
        }
        *last = Some(now_ms);
        true
    }

    /// Whether to log a report from another conversation now: at most once a minute, so a
    /// chatty one cannot fill the log.
    pub fn log_foreign_now(&self, now_ms: u64) -> bool {
        let mut last = lock(&self.foreign_logged_ms);
        if last.is_some_and(|at| now_ms.saturating_sub(at) < FOREIGN_LOG_EVERY_MS) {
            return false;
        }
        *last = Some(now_ms);
        true
    }

    pub fn write_to_agent(&self, bytes: &[u8]) {
        let mut writer = lock(&self.writer);
        let _ = writer.write_all(bytes);
        let _ = writer.flush();
    }

    /// Owner input from an attached client.
    /// Input from an attached viewer: typing puts that viewer in use; the terminal's own
    /// replies to the agent's queries do not.
    pub fn input_from(&self, viewer_id: u64, bytes: &[u8], now_ms: u64) {
        if !is_terminal_report(bytes) {
            self.focus(viewer_id);
        }
        self.input(bytes, now_ms);
    }

    /// The viewer brought the session to the front: the terminal takes its size.
    pub fn focus(&self, viewer_id: u64) {
        let new_size = {
            let viewers = lock(&self.viewers);
            let Some(viewer) = viewers.iter().find(|viewer| viewer.id == viewer_id) else {
                return;
            };
            let mut in_use = lock(&self.in_use);
            let was = in_use.active;
            in_use.use_viewer(viewer_id);
            let changed = was != Some(viewer_id);
            if changed {
                announce_in_use(&viewers, in_use.active);
            }
            changed.then_some((viewer.cols, viewer.rows))
        };
        if let Some((cols, rows)) = new_size {
            self.set_size(cols, rows);
        }
    }

    /// The paired phone the terminal is sized for, while a phone has it.
    pub fn phone_in_use(&self) -> Option<String> {
        let viewers = lock(&self.viewers);
        let active = lock(&self.in_use).active?;
        viewers
            .iter()
            .find(|viewer| viewer.id == active)
            .and_then(|viewer| viewer.device.clone())
    }

    pub fn input(&self, bytes: &[u8], now_ms: u64) {
        self.write_to_agent(bytes);
        if !is_terminal_report(bytes) {
            self.apply(EventKind::Seen, now_ms);
        }
    }

    pub fn apply(&self, event: EventKind, now_ms: u64) {
        lock(&self.tracker).apply(event, now_ms);
    }

    /// An event the agent reported with OSC 7501, which is trusted first.
    pub fn apply_program_status(&self, event: EventKind, now_ms: u64) {
        self.program_status.store(true, Ordering::SeqCst);
        self.has_spoken.store(true, Ordering::SeqCst);
        self.apply(event, now_ms);
    }

    /// Whether the agent has reported anything itself, through OSC 7501 or a hook: it
    /// is up (add-inactive-sessions D3).
    pub fn has_reported(&self) -> bool {
        self.has_spoken.load(Ordering::SeqCst)
    }

    /// An event from the agent's hooks. Once the agent reports its own status, tool
    /// hooks only supplement it: Claude Code can deliver a tool hook after its turn has
    /// ended, which must not mark the session working again.
    pub fn apply_hook(&self, event: EventKind, now_ms: u64) {
        self.has_spoken.store(true, Ordering::SeqCst);
        if event == EventKind::ToolActivity && self.program_status.load(Ordering::SeqCst) {
            return;
        }
        self.apply(event, now_ms);
    }

    /// The owner looked at the session: what finished is read, nothing is asked.
    pub fn mark_viewed(&self) {
        lock(&self.tracker).viewed();
    }

    pub fn status(&self) -> Status {
        lock(&self.tracker).status()
    }

    /// Marks the session as closing; false when a close had already started.
    pub fn begin_close(&self) -> bool {
        !self.closing.swap(true, Ordering::SeqCst)
    }

    pub fn closing(&self) -> bool {
        self.closing.load(Ordering::SeqCst)
    }

    pub fn set_background(&self, busy: bool) {
        lock(&self.tracker).set_background(busy);
    }

    pub fn tick(&self, now_ms: u64) {
        lock(&self.tracker).tick(now_ms);
    }

    /// The current status when it differs from the last one reported, with that
    /// earlier status.
    pub fn status_change(&self) -> Option<StatusChange> {
        let (current, engaged) = {
            let tracker = lock(&self.tracker);
            let status = tracker.status();
            let reason = (status == Status::NeedsYou)
                .then(|| tracker.needs_reason())
                .flatten();
            ((status, reason), tracker.engaged())
        };
        let mut reported = lock(&self.reported);
        if *reported == Some(current) {
            return None;
        }
        let previous = reported.replace(current).map(|(status, _)| status);
        Some(StatusChange {
            status: current.0,
            reason: current.1,
            previous,
            engaged,
        })
    }

    /// Records agent output for late attachers and sends it to current viewers.
    /// Lock order is buffer, then viewers, here and in `attach`, so a viewer sees each
    /// byte exactly once.
    pub fn output(&self, bytes: &[u8]) {
        let mut buffer = lock(&self.buffer);
        let CatchUp { bytes: kept, modes } = &mut *buffer;
        kept.extend(bytes);
        let excess = kept.len().saturating_sub(CATCH_UP_BYTES);
        if excess > 0 {
            let (front, back) = kept.as_slices();
            let from_front = excess.min(front.len());
            modes.observe(&front[..from_front]);
            modes.observe(&back[..excess - from_front]);
            kept.drain(..excess);
        }
        lock(&self.viewers).retain(|viewer| {
            viewer
                .tx
                .send(Outgoing::Frame(Frame::Output(bytes.to_vec())))
                .is_ok()
        });
        drop(buffer);
        self.note_repaint(bytes);
    }

    /// While a redraw waits, the agent erasing its screen is its repaint at the smaller size.
    fn note_repaint(&self, bytes: &[u8]) {
        let mut redraw = lock(&self.redraw);
        if redraw.restore.is_some()
            && !redraw.repainted
            && bytes
                .windows(ERASE_SCREEN.len())
                .any(|window| window == ERASE_SCREEN)
        {
            redraw.repainted = true;
            self.repainted.notify_all();
        }
    }

    /// Adds a viewer: sends the catch-up buffer, then resizes the PTY so the agent
    /// redraws at the viewer's size.
    /// Adds a viewer, which is then in use: the agent redraws at its size.
    pub fn attach(self: &Arc<Self>, viewer: Viewer) {
        let (cols, rows) = (viewer.cols, viewer.rows);
        let id = viewer.id;
        {
            // Hold the buffer lock while registering so no output falls between the
            // catch-up copy and live forwarding.
            let buffer = lock(&self.buffer);
            let preamble = buffer.modes.preamble();
            if !preamble.is_empty() {
                let _ = viewer.tx.send(Outgoing::Frame(Frame::Output(preamble)));
            }
            let (front, back) = buffer.bytes.as_slices();
            for part in [front, back] {
                for chunk in part.chunks(CATCH_UP_CHUNK) {
                    let _ = viewer
                        .tx
                        .send(Outgoing::Frame(Frame::Output(chunk.to_vec())));
                }
            }
            let viewers = &mut *lock(&self.viewers);
            viewers.push(viewer);
            let mut in_use = lock(&self.in_use);
            in_use.use_viewer(id);
            announce_in_use(viewers, in_use.active);
        }
        self.redraw_at(cols, rows);
        lock(&self.tracker).viewed();
    }

    /// Removes a viewer. When it was in use, the most recently used viewer still attached
    /// takes over and the agent redraws at its size, so no screen is left misdrawn.
    pub fn detach(self: &Arc<Self>, viewer_id: u64) {
        let next = {
            let mut viewers = lock(&self.viewers);
            let before = viewers.len();
            viewers.retain(|viewer| viewer.id != viewer_id);
            let removed = viewers.len() < before;
            let mut in_use = lock(&self.in_use);
            in_use.order.retain(|known| *known != viewer_id);
            if !removed || in_use.active.is_some_and(|active| active != viewer_id) {
                None
            } else {
                let next = in_use
                    .order
                    .iter()
                    .rev()
                    .find_map(|id| viewers.iter().find(|viewer| viewer.id == *id))
                    .or_else(|| viewers.last());
                in_use.active = next.map(|viewer| viewer.id);
                announce_in_use(&viewers, in_use.active);
                next.map(|viewer| (viewer.cols, viewer.rows))
            }
        };
        if let Some((cols, rows)) = next {
            self.redraw_at(cols, rows);
        }
    }

    /// A viewer's new size; it applies now only when that viewer is in use, and is kept
    /// for when it is (add-remote-gateway D6).
    pub fn resize(&self, viewer_id: u64, cols: u16, rows: u16) {
        let applies = {
            let mut viewers = lock(&self.viewers);
            let Some(viewer) = viewers.iter_mut().find(|viewer| viewer.id == viewer_id) else {
                return;
            };
            // The same size again changes nothing, and must not cut a redraw short.
            if (viewer.cols, viewer.rows) == (cols, rows) {
                return;
            }
            viewer.cols = cols;
            viewer.rows = rows;
            lock(&self.in_use)
                .active
                .is_none_or(|active| active == viewer_id)
        };
        if applies {
            self.set_size(cols, rows);
        }
    }

    /// Sizes the agent's terminal now, replacing any redraw in progress.
    fn set_size(&self, cols: u16, rows: u16) {
        let mut redraw = lock(&self.redraw);
        redraw.generation += 1;
        redraw.restore = None;
        let _ = lock(&self.master).resize(size(cols, rows));
    }

    /// Makes the agent repaint at `cols` x `rows`, even when that is the size it has: one row
    /// fewer first, then the size again once the agent has repainted at the smaller size, or
    /// after `REDRAW_WAIT`. Both steps at once can reach a busy agent as no change at all,
    /// and it then never repaints (fix-reattach-redraw).
    fn redraw_at(self: &Arc<Self>, cols: u16, rows: u16) {
        let generation = {
            let mut redraw = lock(&self.redraw);
            redraw.generation += 1;
            redraw.restore = Some((cols, rows));
            redraw.repainted = false;
            let _ = lock(&self.master).resize(size(cols, rows.saturating_sub(1).max(1)));
            redraw.generation
        };
        let hosted = Arc::clone(self);
        std::thread::spawn(move || hosted.finish_redraw(generation));
    }

    fn finish_redraw(&self, generation: u64) {
        let deadline = Instant::now() + REDRAW_WAIT;
        let mut redraw = lock(&self.redraw);
        while redraw.generation == generation && !redraw.repainted {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            redraw = self
                .repainted
                .wait_timeout(redraw, left)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
        }
        if redraw.generation != generation {
            return;
        }
        if let Some((cols, rows)) = redraw.restore.take() {
            let _ = lock(&self.master).resize(size(cols, rows));
        }
    }

    /// Tells viewers the agent ended and closes their attach.
    pub fn end_viewers(&self, exit_code: Option<u32>) {
        let ended = Response::Ended {
            session: self.id.clone(),
            exit_code,
        };
        for viewer in lock(&self.viewers).drain(..) {
            let _ = viewer.tx.send(Outgoing::Frame(Frame::response(&ended)));
        }
    }

    pub fn kill(&self) {
        let _ = lock(&self.killer).kill();
    }
}

fn size(cols: u16, rows: u16) -> PtySize {
    PtySize {
        rows: rows.max(1),
        cols: cols.max(1),
        pixel_width: 0,
        pixel_height: 0,
    }
}
