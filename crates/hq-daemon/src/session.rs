//! One hosted agent session: its PTY, catch-up buffer, attached viewers, and status.
//! Terminal bytes live only in memory here and are never written to disk or the log.

use std::collections::VecDeque;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard};

use hq_protocol::{Frame, Response};
use hq_signals::event::{Agent, EventKind, NeedsReason};
use hq_signals::helpers::is_terminal_report;
use hq_signals::status::{SessionTracker, Status};
use portable_pty::{ChildKiller, MasterPty, PtySize};

use crate::modes::Modes;

/// Output kept for clients that attach later.
pub const CATCH_UP_BYTES: usize = 1024 * 1024;
const CATCH_UP_CHUNK: usize = 64 * 1024;

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
            tracker: Mutex::new(parts.tracker),
            reported: Mutex::new(None),
            closing: AtomicBool::new(false),
            program_status: AtomicBool::new(false),
            has_spoken: AtomicBool::new(false),
            conversation_known: Mutex::new(parts.conversation_known),
        }
    }

    pub fn write_to_agent(&self, bytes: &[u8]) {
        let mut writer = lock(&self.writer);
        let _ = writer.write_all(bytes);
        let _ = writer.flush();
    }

    /// Owner input from an attached client.
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
    }

    /// Adds a viewer: sends the catch-up buffer, then resizes the PTY so the agent
    /// redraws at the viewer's size.
    pub fn attach(&self, viewer: Viewer) {
        let (cols, rows) = (viewer.cols, viewer.rows);
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
            lock(&self.viewers).push(viewer);
        }
        self.redraw_at(cols, rows);
        lock(&self.tracker).viewed();
    }

    /// Removes a viewer. The agent then redraws at the size of the latest viewer still
    /// attached, so a client that attached at another size leaves no misdrawn screen.
    pub fn detach(&self, viewer_id: u64) {
        let remaining = {
            let mut viewers = lock(&self.viewers);
            let before = viewers.len();
            viewers.retain(|viewer| viewer.id != viewer_id);
            let removed = viewers.len() < before;
            viewers
                .last()
                .filter(|_| removed)
                .map(|viewer| (viewer.cols, viewer.rows))
        };
        if let Some((cols, rows)) = remaining {
            self.redraw_at(cols, rows);
        }
    }

    pub fn resize(&self, viewer_id: u64, cols: u16, rows: u16) {
        if let Some(viewer) = lock(&self.viewers)
            .iter_mut()
            .find(|viewer| viewer.id == viewer_id)
        {
            viewer.cols = cols;
            viewer.rows = rows;
        }
        let _ = lock(&self.master).resize(size(cols, rows));
    }

    /// A same-size resize sends no SIGWINCH, so step through a different size to make
    /// full-screen agents repaint.
    fn redraw_at(&self, cols: u16, rows: u16) {
        let master = lock(&self.master);
        let _ = master.resize(size(cols, rows.saturating_sub(1).max(1)));
        let _ = master.resize(size(cols, rows));
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
