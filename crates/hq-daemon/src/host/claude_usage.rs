//! The hidden `/usage` read (`refresh-usage-on-activity` design D2): Claude Code runs in
//! a terminal no one sees, with only its project and local settings so the owner's
//! user-level hooks and status line stay out of it, in a folder Claude has already been
//! trusted with. Once it reports itself idle, `/usage` is typed, the output is drawn
//! into a terminal grid, and the screen's text is parsed. Claude is then hung up. A
//! folder-trust question is never answered: the read stops and counts as a failure.
//! Nothing read here is stored but the parsed numbers.

use std::io::Write;
use std::path::Path;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line};
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};
use chrono::Local;
use hq_signals::claude_usage::parse_usage_screen;
use hq_signals::quota::QuotaWindow;
use hq_signals::scanner::{PROGRAM_STATUS_SUPPORT_REPLY, ProgramState, Scanner, TerminalSignal};
use hq_signals::{adapters, external};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

/// Wide and tall enough that the whole usage block is on screen.
const COLUMNS: u16 = 120;
const ROWS: u16 = 80;
/// How long Claude may take to start and report itself idle.
const READY_WAIT: Duration = Duration::from_secs(20);
/// How long the `/usage` screen may take to show its numbers.
const SCREEN_WAIT: Duration = Duration::from_secs(12);
/// The per-model blocks come a moment after the first ones.
const SETTLE: Duration = Duration::from_millis(700);
const KEY_GAP: Duration = Duration::from_millis(15);
const EXIT_WAIT: Duration = Duration::from_secs(2);
/// Words of Claude Code's folder-trust question.
const TRUST_QUESTION: &str = "trust this folder";
/// The read's error when Claude Code asked to trust its folder; the owner can act on it.
pub(super) const TRUST_ASKED: &str =
    "Claude Code asked whether to trust the folder; Headquarter does not answer it";

pub(super) fn read_usage_screen(claude: &Path, folder: &Path) -> Result<Vec<QuotaWindow>, String> {
    let pair = native_pty_system()
        .openpty(PtySize {
            rows: ROWS,
            cols: COLUMNS,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|error| format!("cannot open a pseudo-terminal: {error}"))?;
    let mut command = CommandBuilder::new(claude);
    command.args(["--setting-sources", "project,local"]);
    command.cwd(folder);
    for marker in adapters::PARENT_SESSION_MARKERS {
        command.env_remove(marker);
    }
    command.env("TERM", "xterm-256color");
    command.env("PATH", hq_discovery::shell::agent_path(claude));
    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| format!("cannot start Claude Code: {error}"))?;
    drop(pair.slave);
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| format!("pty reader: {error}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|error| format!("pty writer: {error}"))?;

    let result = drive(reader, writer);

    // Hang Claude up; it saved nothing, so there is nothing to wait for.
    let _ = child.kill();
    let deadline = Instant::now() + EXIT_WAIT;
    while Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return result;
        }
        thread::sleep(Duration::from_millis(50));
    }
    if let Some(pid) = child.process_id() {
        external::force_kill(pid);
    }
    let _ = child.wait();
    result
}

fn drive(
    mut reader: Box<dyn std::io::Read + Send>,
    mut writer: Box<dyn Write + Send>,
) -> Result<Vec<QuotaWindow>, String> {
    let (chunks_tx, chunks) = mpsc::channel::<Vec<u8>>();
    // Keeps reading after the read is done: an exiting process waits until its
    // terminal output has been read, so Claude would never finish exiting. The thread
    // ends when the terminal closes.
    thread::spawn(move || {
        let mut buffer = [0_u8; 16 * 1024];
        while let Ok(count) = reader.read(&mut buffer) {
            if count == 0 {
                break;
            }
            let _ = chunks_tx.send(buffer[..count].to_vec());
        }
    });
    let mut screen = Screen::new();
    let mut scanner = Scanner::new();
    let send = |writer: &mut Box<dyn Write + Send>, bytes: &[u8]| {
        let _ = writer.write_all(bytes);
        let _ = writer.flush();
    };

    // Until Claude reports itself idle, answering the terminal's questions on the way.
    let deadline = Instant::now() + READY_WAIT;
    let mut ready = false;
    while !ready {
        let chunk = next(&chunks, deadline).ok_or("Claude Code did not get ready in time")?;
        for signal in scanner.feed(&chunk) {
            match signal {
                TerminalSignal::ProgramStatusQuery => {
                    send(&mut writer, PROGRAM_STATUS_SUPPORT_REPLY);
                }
                TerminalSignal::ProgramStatus(status)
                    if status.id.is_none() && status.state == ProgramState::Idle =>
                {
                    ready = true;
                }
                _ => {}
            }
        }
        send(&mut writer, &screen.feed(&chunk));
        if screen.text().to_lowercase().contains(TRUST_QUESTION) {
            return Err(TRUST_ASKED.to_owned());
        }
    }

    for key in "/usage".bytes() {
        send(&mut writer, &[key]);
        thread::sleep(KEY_GAP);
    }
    thread::sleep(KEY_GAP * 10);
    send(&mut writer, b"\r");

    let deadline = Instant::now() + SCREEN_WAIT;
    let mut shown_at: Option<Instant> = None;
    loop {
        if let Some(chunk) = next(&chunks, Instant::now() + Duration::from_millis(200)) {
            let _ = scanner.feed(&chunk);
            send(&mut writer, &screen.feed(&chunk));
        }
        let parsed = parse_usage_screen(&screen.lines(), &Local::now());
        match (&parsed, shown_at) {
            (Ok(_), None) => shown_at = Some(Instant::now()),
            (Ok(_), Some(at)) if at.elapsed() >= SETTLE => return parsed,
            _ => {}
        }
        if Instant::now() >= deadline {
            return parsed;
        }
    }
}

fn next(chunks: &mpsc::Receiver<Vec<u8>>, deadline: Instant) -> Option<Vec<u8>> {
    chunks
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .ok()
}

/// The hidden terminal: Claude's output drawn into a grid, and the replies the terminal
/// owes it (cursor position and the like).
struct Screen {
    term: Term<Replies>,
    processor: Processor<StdSyncHandler>,
    replies: Arc<Mutex<Vec<u8>>>,
}

#[derive(Clone)]
struct Replies(Arc<Mutex<Vec<u8>>>);

impl EventListener for Replies {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(text) = event {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend(text.into_bytes());
        }
    }
}

struct Size;

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        usize::from(ROWS)
    }

    fn screen_lines(&self) -> usize {
        usize::from(ROWS)
    }

    fn columns(&self) -> usize {
        usize::from(COLUMNS)
    }
}

impl Screen {
    fn new() -> Screen {
        let replies = Arc::new(Mutex::new(Vec::new()));
        Screen {
            term: Term::new(Config::default(), &Size, Replies(Arc::clone(&replies))),
            processor: Processor::new(),
            replies,
        }
    }

    /// Draws `bytes` and returns what the terminal must answer.
    fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.processor.advance(&mut self.term, bytes);
        std::mem::take(
            &mut *self
                .replies
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    fn lines(&self) -> Vec<String> {
        let grid = self.term.grid();
        (0..usize::from(ROWS))
            .map(|row| {
                let line = &grid[Line(i32::try_from(row).unwrap_or(0))];
                (0..usize::from(COLUMNS))
                    .map(|column| line[Column(column)].c)
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    fn text(&self) -> String {
        self.lines().join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_is_drawn_and_terminal_questions_are_answered() {
        let mut screen = Screen::new();
        let replies = screen.feed(b"Current session\r\n\x1b[2;30H15%used\x1b[6n");
        assert_eq!(screen.lines()[0], "Current session");
        assert!(screen.lines()[1].ends_with("15%used"));
        assert!(
            String::from_utf8_lossy(&replies).starts_with("\x1b["),
            "the cursor position is answered"
        );
    }
}
