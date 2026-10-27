//! One attached session: a connection to the host, the terminal model it feeds, and the
//! way back for input, replies and size changes. The host keeps the agent running when
//! this is dropped; only this view's attachment ends.

use std::cell::Cell;
use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use alacritty_terminal::term::ClipboardType;
use futures::StreamExt;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use gpui::{App, ClipboardItem, Context, Task};
use hq_client::HostPaths;
use hq_protocol::{Frame, Request, Response, SessionInfo, read_frame, write_frame};

use super::model::{GridSize, TerminalModel};
use crate::theme::Palette;

/// The size to attach at when the pane is never laid out (fix-reattach-redraw).
const INITIAL_SIZE: GridSize = GridSize {
    columns: 80,
    rows: 24,
};

/// How long to wait for the pane's first layout before attaching at `INITIAL_SIZE`.
const LAYOUT_WAIT: Duration = Duration::from_millis(500);

enum Incoming {
    Connected(UnixStream),
    Output(Vec<u8>),
    Ended(Option<u32>),
    Failed(String),
}

pub struct TerminalSession {
    pub info: SessionInfo,
    model: TerminalModel,
    writer: Option<Arc<Mutex<UnixStream>>>,
    /// Why the pane shows no live agent: it ended, or the connection failed.
    pub ended: Option<String>,
    /// How many copies the agent has made (OSC 52), and the note for the latest, so
    /// the pane can say "Copied" for them too.
    pub agent_copies: (u64, String),
    /// When the owner last sent this session input: keys, a paste or a mouse report.
    last_input: Cell<Option<Instant>>,
    /// Attached to the session's shell rather than its agent (add-session-shell-panel).
    pub shell: bool,
    /// The connection's way back, until it starts: at the pane's first size, so the agent
    /// never draws at a size the pane does not show (fix-reattach-redraw).
    unstarted: Option<UnboundedSender<Incoming>>,
    _reader: Task<()>,
    _layout_wait: Task<()>,
}

/// How long after the owner's input an agent's copy is still theirs.
const AGENT_COPY_WINDOW: Duration = Duration::from_secs(2);

/// Whether an agent's copy follows the owner's input closely enough to be what they
/// asked for. Claude Code sends its last selection again whenever it redraws, as when
/// a session is opened; such a copy must not take the clipboard
/// (fix-agent-copy-after-input).
fn accepts_agent_copy(last_input: Option<Instant>, now: Instant) -> bool {
    last_input.is_some_and(|at| now.saturating_duration_since(at) <= AGENT_COPY_WINDOW)
}

impl TerminalSession {
    pub fn attach(info: SessionInfo, palette: Palette, cx: &mut Context<Self>) -> Self {
        Self::connect(info, palette, false, cx)
    }

    /// Attaches to the session's shell, which the host starts if none runs.
    pub fn attach_shell(info: SessionInfo, palette: Palette, cx: &mut Context<Self>) -> Self {
        Self::connect(info, palette, true, cx)
    }

    fn connect(info: SessionInfo, palette: Palette, shell: bool, cx: &mut Context<Self>) -> Self {
        let (tx, mut rx) = unbounded();
        let layout_wait = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(LAYOUT_WAIT).await;
            let _ = this.update(cx, |session, _| session.start(INITIAL_SIZE));
        });
        let reader = cx.spawn(async move |this, cx| {
            while let Some(incoming) = rx.next().await {
                if this
                    .update(cx, |session, cx| session.receive(incoming, cx))
                    .is_err()
                {
                    break;
                }
            }
        });
        TerminalSession {
            info,
            model: TerminalModel::new(INITIAL_SIZE, palette),
            writer: None,
            ended: None,
            agent_copies: (0, String::new()),
            last_input: Cell::new(None),
            shell,
            unstarted: Some(tx),
            _reader: reader,
            _layout_wait: layout_wait,
        }
    }

    /// Connects and attaches at `size`, once.
    fn start(&mut self, size: GridSize) {
        let Some(tx) = self.unstarted.take() else {
            return;
        };
        let session_id = self.info.id.clone();
        let shell = self.shell;
        let (cols, rows) = (clamp(size.columns), clamp(size.rows));
        std::thread::spawn(move || run_connection(&session_id, shell, cols, rows, &tx));
    }

    fn receive(&mut self, incoming: Incoming, cx: &mut Context<Self>) {
        match incoming {
            Incoming::Connected(stream) => {
                self.writer = Some(Arc::new(Mutex::new(stream)));
                let size = self.model.size();
                self.send(&Frame::request(&Request::Resize {
                    cols: clamp(size.columns),
                    rows: clamp(size.rows),
                }));
            }
            Incoming::Output(bytes) => {
                let replies = self.model.feed(&bytes);
                if !replies.is_empty() {
                    self.send(&Frame::Input(replies));
                }
                for (target, text) in self.model.take_copies() {
                    if !accepts_agent_copy(self.last_input.get(), Instant::now()) {
                        log::debug!("agent copy ignored: no input from the owner just before");
                        continue;
                    }
                    self.agent_copies = (self.agent_copies.0 + 1, super::copy::copied_note(&text));
                    store_copy(target, text, cx);
                }
            }
            Incoming::Ended(code) => {
                let code = code.map_or_else(|| "unknown".to_owned(), |code| code.to_string());
                let program = if self.shell {
                    "The shell".to_owned()
                } else {
                    self.info.agent.to_string()
                };
                self.ended = Some(format!("{program} exited (code {code})"));
                self.writer = None;
            }
            Incoming::Failed(message) => {
                self.ended = Some(message);
                self.writer = None;
            }
        }
        cx.notify();
    }

    fn send(&self, frame: &Frame) {
        if let Some(writer) = &self.writer
            && let Ok(mut stream) = writer.lock()
        {
            // A failed write means the host went away; the reader reports it.
            let _ = write_frame(&mut *stream, frame);
        }
    }

    /// Owner input: keys, pasted text, composed characters.
    pub fn input(&mut self, bytes: &[u8], cx: &mut Context<Self>) {
        if bytes.is_empty() {
            return;
        }
        self.model.scroll_to_bottom();
        self.model.clear_selection();
        self.last_input.set(Some(Instant::now()));
        self.send(&Frame::Input(bytes.to_vec()));
        cx.notify();
    }

    /// Bytes that are not owner input, such as mouse reports.
    pub fn report(&self, bytes: &[u8]) {
        self.last_input.set(Some(Instant::now()));
        self.send(&Frame::Input(bytes.to_vec()));
    }

    /// The owner brought this session to the front here: the host sizes its terminal for
    /// this window (add-remote-gateway D6).
    pub fn focus(&self) {
        self.send(&Frame::request(&Request::Focus {}));
    }

    pub fn set_size(&mut self, size: GridSize, cx: &mut Context<Self>) {
        if self.unstarted.is_some() && size.columns > 0 && size.rows > 0 {
            self.model.resize(size);
            self.start(size);
            cx.notify();
            return;
        }
        let Some(request) = resize_request(self.model.size(), size) else {
            return;
        };
        self.model.resize(size);
        self.send(&Frame::request(&request));
        cx.notify();
    }

    pub fn model(&self) -> &TerminalModel {
        &self.model
    }

    pub fn model_mut(&mut self) -> &mut TerminalModel {
        &mut self.model
    }
}

/// The `Resize` to send when the pane's grid changes from `old` to `new`.
fn resize_request(old: GridSize, new: GridSize) -> Option<Request> {
    if new == old || new.columns == 0 || new.rows == 0 {
        return None;
    }
    Some(Request::Resize {
        cols: clamp(new.columns),
        rows: clamp(new.rows),
    })
}

fn clamp(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

/// Runs on its own thread: connects, attaches, then forwards frames until the session
/// ends or the connection closes.
fn run_connection(
    session_id: &str,
    shell: bool,
    cols: u16,
    rows: u16,
    tx: &UnboundedSender<Incoming>,
) {
    let fail = |message: String| {
        let _ = tx.unbounded_send(Incoming::Failed(message));
    };
    let Some(paths) = HostPaths::standard() else {
        fail("cannot find the home folder".to_owned());
        return;
    };
    let attached = if shell {
        hq_client::attach_shell(&paths, session_id, cols, rows)
    } else {
        hq_client::attach(&paths, session_id, cols, rows)
    };
    let client = match attached {
        Ok(client) => client,
        Err(error) => return fail(format!("{error:#}")),
    };
    match client.writer.try_clone() {
        Ok(stream) => {
            if tx.unbounded_send(Incoming::Connected(stream)).is_err() {
                return;
            }
        }
        Err(error) => return fail(format!("connection error: {error}")),
    }
    let mut reader: BufReader<UnixStream> = client.reader;
    loop {
        let incoming = match read_frame(&mut reader) {
            Ok(Some(Frame::Output(bytes))) => Incoming::Output(bytes),
            Ok(Some(Frame::Control(bytes))) => match Response::parse(&bytes) {
                Ok(Response::Ended { exit_code, .. }) => Incoming::Ended(exit_code),
                Ok(Response::Error { message, .. }) => Incoming::Failed(message),
                _ => continue,
            },
            Ok(Some(Frame::Input(_))) => continue,
            Ok(None) => Incoming::Failed("the Headquarter host closed the connection".to_owned()),
            Err(error) => Incoming::Failed(error.to_string()),
        };
        let last = matches!(incoming, Incoming::Ended(_) | Incoming::Failed(_));
        if tx.unbounded_send(incoming).is_err() || last {
            return;
        }
    }
}

/// Puts text the agent copied (OSC 52), such as a mouse selection in Claude Code, where
/// Alacritty would: the clipboard, or on Linux the primary selection for that target.
fn store_copy(target: ClipboardType, text: String, cx: &mut App) {
    let item = ClipboardItem::new_string(text);
    match target {
        ClipboardType::Clipboard => cx.write_to_clipboard(item),
        #[cfg(any(target_os = "linux", target_os = "freebsd"))]
        ClipboardType::Selection => cx.write_to_primary(item),
        // macOS has no primary selection; Alacritty ignores this target there too.
        #[cfg(not(any(target_os = "linux", target_os = "freebsd")))]
        ClipboardType::Selection => drop(item),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_copies_count_only_right_after_the_owners_input() {
        let now = Instant::now();
        assert!(
            !accepts_agent_copy(None, now),
            "a redraw on opening, with no input"
        );
        assert!(
            accepts_agent_copy(Some(now - Duration::from_millis(300)), now),
            "drag released"
        );
        assert!(accepts_agent_copy(Some(now - AGENT_COPY_WINDOW), now));
        assert!(
            !accepts_agent_copy(Some(now - Duration::from_secs(30)), now),
            "a re-send long after the last input"
        );
    }

    const SIZE: GridSize = GridSize {
        columns: 80,
        rows: 24,
    };

    #[test]
    fn resize_is_sent_only_on_change() {
        assert!(resize_request(SIZE, SIZE).is_none());
        let wider = GridSize {
            columns: 120,
            ..SIZE
        };
        assert!(matches!(
            resize_request(SIZE, wider),
            Some(Request::Resize {
                cols: 120,
                rows: 24
            })
        ));
        let empty = GridSize { columns: 0, ..SIZE };
        assert!(resize_request(SIZE, empty).is_none());
        let huge = GridSize {
            columns: 100_000,
            ..SIZE
        };
        assert!(matches!(
            resize_request(SIZE, huge),
            Some(Request::Resize { cols: u16::MAX, .. })
        ));
    }
}
