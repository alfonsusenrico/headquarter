//! `hq-probe run`: hosts one agent in a pseudo-terminal, passes its output through
//! unchanged, and records metadata-only events and status changes as JSON lines.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, IsTerminal, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result};
use hq_signals::adapters::{self, Conversation, LaunchContext, OwnerSettings, hook};
use hq_signals::event::{Agent, EventKind, EventRecord, Source};
use hq_signals::forward::ForwardedPayload;
use hq_signals::now_ms;
use hq_signals::scanner::{PROGRAM_STATUS_SUPPORT_REPLY, ProgramState, Scanner, TerminalSignal};
use hq_signals::status::{SessionTracker, Status, Thresholds};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use serde_json::{Value, json};

const TICK: Duration = Duration::from_millis(250);
/// Output after the agent exits is drained until this much quiet.
const DRAIN_QUIET: Duration = Duration::from_millis(300);
/// Output activity is reported at most this often.
const OUTPUT_THROTTLE_MS: u64 = 200;
/// The same signal or event is logged at most this often; spinner titles repeat
/// many times a second.
const LOG_REPEAT_MS: u64 = 5_000;

pub struct RunOptions {
    pub agent: Agent,
    pub cwd: Option<PathBuf>,
    pub log: Option<PathBuf>,
    pub program: Option<String>,
    pub extra: Vec<String>,
}

enum Message {
    Hook(ForwardedPayload),
    Signal(TerminalSignal),
    Output,
    Input,
    Exited(u32),
}

pub fn run(options: RunOptions) -> Result<ExitCode> {
    let agent = options.agent;
    let cwd = match options.cwd {
        Some(cwd) => cwd,
        None => std::env::current_dir().context("current folder")?,
    }
    .canonicalize()
    .context("working folder")?;

    let probe_dir = hq_signals::paths::probe_dir().context("home directory unknown")?;
    std::fs::create_dir_all(&probe_dir).context("probe data folder")?;
    std::fs::set_permissions(&probe_dir, std::fs::Permissions::from_mode(0o700))?;

    let socket = probe_dir.join(format!("{}.sock", std::process::id()));
    let _ = std::fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket).context("hook socket")?;
    let _socket_cleanup = RemoveOnDrop(socket.clone());

    let session_id = uuid::Uuid::new_v4().to_string();
    let helper_exe = std::env::current_exe().context("probe executable path")?;
    let owner = OwnerSettings::discover(&cwd);
    let mut spec = adapters::launch_spec(
        agent,
        &LaunchContext {
            helper_exe: &helper_exe,
            hook_socket: &socket,
            conversation: Conversation::New { id: &session_id },
        },
        &owner,
    );
    let agent_version = match options.program {
        Some(program) => {
            spec.program = program;
            None
        }
        None => agent_version(&spec.program),
    };
    spec.args.extend(options.extra);

    let log_path = options.log.unwrap_or_else(|| {
        probe_dir
            .join("runs")
            .join(format!("{agent}-{}.jsonl", now_ms() / 1000))
    });
    let mut log = EventLog::create(&log_path)?;
    log.write(&json!({
        "type": "launch",
        "at_ms": now_ms(),
        "agent": agent,
        "agent_version": agent_version,
        "session_id": session_id,
        "cwd": cwd,
        "structured": spec.structured,
        "owner_status_line_chained": owner.claude_status_line.is_some(),
    }));

    let (tx, rx) = mpsc::channel::<Message>();
    spawn_hook_listener(listener, tx.clone());

    let pair = native_pty_system()
        .openpty(terminal_size())
        .map_err(|error| anyhow::anyhow!("cannot open a pseudo-terminal: {error}"))?;
    let mut command = CommandBuilder::new(&spec.program);
    command.args(&spec.args);
    command.cwd(&cwd);
    let mut child = pair
        .slave
        .spawn_command(command)
        .map_err(|error| anyhow::anyhow!("cannot start {}: {error}", spec.program))?;
    drop(pair.slave);
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|error| anyhow::anyhow!("pty reader: {error}"))?;
    let writer = Arc::new(Mutex::new(
        pair.master
            .take_writer()
            .map_err(|error| anyhow::anyhow!("pty writer: {error}"))?,
    ));
    let master = pair.master;

    let _raw_mode = RawMode::enable_if_terminal()?;

    spawn_output_pump(reader, Arc::clone(&writer), tx.clone());
    spawn_input_pump(Arc::clone(&writer), tx.clone());
    thread::spawn(move || {
        let Ok(mut signals) = signal_hook::iterator::Signals::new([signal_hook::consts::SIGWINCH])
        else {
            return;
        };
        for _ in signals.forever() {
            let _ = master.resize(terminal_size());
        }
    });
    {
        let tx = tx.clone();
        thread::spawn(move || {
            let code = child.wait().map(|status| status.exit_code()).unwrap_or(1);
            let _ = tx.send(Message::Exited(code));
        });
    }
    drop(tx);

    let mut session = Session {
        agent,
        tracker: SessionTracker::new(spec.structured, Thresholds::default(), now_ms()),
        log,
        last_logged: HashMap::new(),
        last_status: None,
        statuses_seen: Vec::new(),
    };
    session.record_status(now_ms());

    let mut exit_code = None;
    loop {
        let wait = if exit_code.is_some() {
            DRAIN_QUIET
        } else {
            TICK
        };
        match rx.recv_timeout(wait) {
            Ok(Message::Exited(code)) => {
                exit_code = Some(code);
                session.apply(Source::Process, EventKind::Ended, None);
                session
                    .log
                    .write(&json!({"type": "exit", "at_ms": now_ms(), "code": code}));
            }
            Ok(message) => session.handle(message),
            Err(RecvTimeoutError::Timeout) if exit_code.is_some() => break,
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => break,
        }
        let now = now_ms();
        session.tracker.tick(now);
        session.record_status(now);
    }

    drop(_raw_mode);
    let seen: Vec<&str> = session
        .statuses_seen
        .iter()
        .map(|status| status.label())
        .collect();
    eprintln!(
        "\r\nhq-probe: {agent} exited (code {}). Statuses seen: {}.\r\nhq-probe: log {}",
        exit_code.unwrap_or(1),
        seen.join(" → "),
        log_path.display()
    );
    Ok(ExitCode::from(
        u8::try_from(exit_code.unwrap_or(1)).unwrap_or(1),
    ))
}

struct Session {
    agent: Agent,
    tracker: SessionTracker,
    log: EventLog,
    last_logged: HashMap<String, u64>,
    last_status: Option<Status>,
    statuses_seen: Vec<Status>,
}

impl Session {
    fn handle(&mut self, message: Message) {
        let now = now_ms();
        match message {
            Message::Hook(payload) => {
                if payload.agent != self.agent.id() {
                    self.warn("hook payload for another agent");
                    return;
                }
                match hook::normalize(self.agent, &payload.raw, now) {
                    Ok(Some(record)) => {
                        self.tracker.apply(record.event, now);
                        self.log_event(&record);
                    }
                    Ok(None) => {}
                    Err(error) => self.warn(&error.to_string()),
                }
            }
            Message::Signal(signal) => {
                let label = signal_label(&signal);
                if self.should_log(&format!("signal:{label}"), now) {
                    self.log
                        .write(&json!({"type": "signal", "at_ms": now, "signal": label}));
                }
                if let Some((source, event)) = adapters::terminal_event(self.agent, &signal) {
                    self.apply(source, event, None);
                }
            }
            Message::Output => self.tracker.apply(EventKind::OutputActivity, now),
            Message::Input => self.tracker.apply(EventKind::Seen, now),
            Message::Exited(_) => {}
        }
    }

    fn apply(&mut self, source: Source, event: EventKind, session_id: Option<String>) {
        let now = now_ms();
        self.tracker.apply(event, now);
        let mut record = EventRecord::new(now, self.agent, source, event);
        record.session_id = session_id;
        self.log_event(&record);
    }

    fn log_event(&mut self, record: &EventRecord) {
        let key = format!("event:{:?}:{:?}", record.source, record.event);
        if self.should_log(&key, record.at_ms)
            && let Ok(Value::Object(mut fields)) = serde_json::to_value(record)
        {
            fields.insert("type".to_owned(), json!("event"));
            self.log.write(&Value::Object(fields));
        }
    }

    fn should_log(&mut self, key: &str, now: u64) -> bool {
        match self.last_logged.get(key) {
            Some(&last) if now.saturating_sub(last) < LOG_REPEAT_MS => false,
            _ => {
                self.last_logged.insert(key.to_owned(), now);
                true
            }
        }
    }

    fn warn(&mut self, warning: &str) {
        self.log
            .write(&json!({"type": "warning", "at_ms": now_ms(), "warning": warning}));
    }

    fn record_status(&mut self, now: u64) {
        let status = self.tracker.status();
        if self.last_status == Some(status) {
            return;
        }
        self.last_status = Some(status);
        self.statuses_seen.push(status);
        // Events seen during the new status are worth logging again.
        self.last_logged.clear();
        self.log.write(&json!({
            "type": "status",
            "at_ms": now,
            "status": status,
            "reason": self.tracker.needs_reason(),
        }));
    }
}

/// A metadata-only description of a signal; titles and notification text are
/// never written.
fn signal_label(signal: &TerminalSignal) -> String {
    match signal {
        TerminalSignal::Title(_) => "title".to_owned(),
        TerminalSignal::Notification { .. } => "notification".to_owned(),
        TerminalSignal::Progress { state, .. } => format!("progress:{state}"),
        TerminalSignal::ProgramStatus(status) => {
            let state = match status.state {
                ProgramState::Idle => "idle",
                ProgramState::Working => "working",
                ProgramState::Done => "done",
                ProgramState::Blocked => "blocked",
                ProgramState::Error => "error",
                ProgramState::Clear => "clear",
            };
            let child = if status.id.is_some() { ":child" } else { "" };
            match status.kind {
                Some(kind) => format!("program-status:{state}:{kind:?}{child}").to_lowercase(),
                None => format!("program-status:{state}{child}"),
            }
        }
        TerminalSignal::ProgramStatusQuery => "program-status-query".to_owned(),
        TerminalSignal::Bell => "bell".to_owned(),
    }
}

fn spawn_hook_listener(listener: UnixListener, tx: mpsc::Sender<Message>) {
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            for line in BufReader::new(stream).lines().map_while(Result::ok) {
                let Ok(payload) = serde_json::from_str::<ForwardedPayload>(&line) else {
                    continue;
                };
                if tx.send(Message::Hook(payload)).is_err() {
                    return;
                }
            }
        }
    });
}

type SharedWriter = Arc<Mutex<Box<dyn Write + Send>>>;

fn spawn_output_pump(
    mut reader: Box<dyn Read + Send>,
    writer: SharedWriter,
    tx: mpsc::Sender<Message>,
) {
    thread::spawn(move || {
        let mut scanner = Scanner::new();
        let stdout = std::io::stdout();
        let mut buffer = [0u8; 8192];
        let mut last_output_ms = 0;
        loop {
            let count = match reader.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(count) => count,
            };
            let signals = {
                let mut out = stdout.lock();
                let signals = scanner.pass_through(&buffer[..count], &mut out);
                let _ = out.flush();
                signals
            };
            let Ok(signals) = signals else { return };
            for signal in signals {
                if signal == TerminalSignal::ProgramStatusQuery
                    && let Ok(mut pty) = writer.lock()
                {
                    let _ = pty.write_all(PROGRAM_STATUS_SUPPORT_REPLY);
                    let _ = pty.flush();
                }
                if tx.send(Message::Signal(signal)).is_err() {
                    return;
                }
            }
            let now = now_ms();
            if now.saturating_sub(last_output_ms) >= OUTPUT_THROTTLE_MS {
                last_output_ms = now;
                if tx.send(Message::Output).is_err() {
                    return;
                }
            }
        }
    });
}

fn spawn_input_pump(writer: SharedWriter, tx: mpsc::Sender<Message>) {
    thread::spawn(move || {
        let mut stdin = std::io::stdin();
        let mut buffer = [0u8; 1024];
        loop {
            let count = match stdin.read(&mut buffer) {
                Ok(0) | Err(_) => return,
                Ok(count) => count,
            };
            {
                let Ok(mut pty) = writer.lock() else { return };
                if pty.write_all(&buffer[..count]).is_err() {
                    return;
                }
                let _ = pty.flush();
            }
            if !hq_signals::helpers::is_terminal_report(&buffer[..count])
                && tx.send(Message::Input).is_err()
            {
                return;
            }
        }
    });
}

fn terminal_size() -> PtySize {
    let (cols, rows) = crossterm::terminal::size().unwrap_or((80, 24));
    PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn agent_version(program: &str) -> Option<String> {
    let output = Command::new(program)
        .arg("--version")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().next().map(|line| line.trim().to_owned())
}

struct EventLog {
    file: File,
}

impl EventLog {
    fn create(path: &Path) -> Result<EventLog> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).context("log folder")?;
        }
        let file = File::create(path).with_context(|| format!("log {}", path.display()))?;
        Ok(EventLog { file })
    }

    /// Evidence logging is best effort; a full disk must not break the agent session.
    fn write(&mut self, record: &Value) {
        let _ = writeln!(self.file, "{record}");
        let _ = self.file.flush();
    }
}

struct RawMode;

impl RawMode {
    fn enable_if_terminal() -> Result<Option<RawMode>> {
        if !std::io::stdin().is_terminal() {
            return Ok(None);
        }
        crossterm::terminal::enable_raw_mode().context("raw mode")?;
        Ok(Some(RawMode))
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
    }
}

struct RemoveOnDrop(PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
