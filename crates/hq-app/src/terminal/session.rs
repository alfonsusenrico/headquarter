//! One attached session: a connection to the host, the terminal model it feeds, and the
//! way back for input, replies and size changes. The host keeps the agent running when
//! this is dropped; only this view's attachment ends.

use std::io::BufReader;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};

use futures::StreamExt;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use gpui::{Context, Task};
use hq_client::HostPaths;
use hq_protocol::{Frame, Request, Response, SessionInfo, read_frame, write_frame};

use super::model::{GridSize, TerminalModel};
use crate::theme::Palette;

/// Size the agent sees until the pane has been laid out.
const INITIAL_SIZE: GridSize = GridSize {
    columns: 80,
    rows: 24,
};

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
    _reader: Task<()>,
}

impl TerminalSession {
    pub fn attach(info: SessionInfo, palette: Palette, cx: &mut Context<Self>) -> Self {
        let (tx, mut rx) = unbounded();
        let session_id = info.id.clone();
        std::thread::spawn(move || run_connection(&session_id, &tx));
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
            _reader: reader,
        }
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
            }
            Incoming::Ended(code) => {
                let code = code.map_or_else(|| "unknown".to_owned(), |code| code.to_string());
                self.ended = Some(format!("{} exited (code {code})", self.info.agent));
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
        self.send(&Frame::Input(bytes.to_vec()));
        cx.notify();
    }

    /// Bytes that are not owner input, such as mouse reports.
    pub fn report(&self, bytes: &[u8]) {
        self.send(&Frame::Input(bytes.to_vec()));
    }

    pub fn set_size(&mut self, size: GridSize, cx: &mut Context<Self>) {
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
fn run_connection(session_id: &str, tx: &UnboundedSender<Incoming>) {
    let fail = |message: String| {
        let _ = tx.unbounded_send(Incoming::Failed(message));
    };
    let Some(paths) = HostPaths::standard() else {
        fail("cannot find the home folder".to_owned());
        return;
    };
    let client = match hq_client::attach(
        &paths,
        session_id,
        clamp(INITIAL_SIZE.columns),
        clamp(INITIAL_SIZE.rows),
    ) {
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

#[cfg(test)]
mod tests {
    use super::*;

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
