//! Checking an agent after it updates (add-agent-update-check): the host notices a new
//! version and checks, without a prompt, that the agent still reports its status as it
//! starts and that its usage still reads. Results are logged, kept per version in
//! `agent-checks.json`, and a failed one travels with the agent's usage so the app can say
//! so. Nothing the agents show is kept or logged.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use hq_protocol::{AgentCheck, CheckResult};
use hq_signals::event::Agent;
use hq_signals::now_ms;
use hq_signals::quota::{self, QuotaReport};
use hq_signals::scanner::{PROGRAM_STATUS_SUPPORT_REPLY, Scanner, TerminalSignal};
use hq_signals::{adapters, external};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};

use super::claude_usage::{self, Screen};
use super::{Host, lock};

/// How long an agent may take to report its status as it starts (D2).
const START_WAIT: Duration = Duration::from_secs(20);
/// Once a folder-trust question is on screen, how long a report may still come: agy
/// reports while it asks, Codex does not report until it is answered.
const AFTER_TRUST_WAIT: Duration = Duration::from_secs(3);
const VERSION_TIMEOUT: Duration = Duration::from_secs(5);
/// Hung up, an agent gets this long to exit before it is killed; agy leaves a crash log
/// when killed.
const EXIT_WAIT: Duration = Duration::from_secs(5);
const CODEX_TIMEOUT: Duration = Duration::from_secs(15);
const AGY_TIMEOUT: Duration = Duration::from_secs(30);
const COLUMNS: u16 = 120;
const ROWS: u16 = 50;

/// What the host saw of each agent's executable, and the checks recorded per agent.
#[derive(Default)]
pub(super) struct CheckBook {
    seen: HashMap<Agent, Fingerprint>,
    recorded: Option<Vec<AgentCheck>>,
}

/// An executable as found: a new version changes at least one of these.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Fingerprint {
    path: PathBuf,
    modified: Option<SystemTime>,
    size: u64,
}

fn fingerprint(executable: &Path) -> Option<Fingerprint> {
    let path = std::fs::canonicalize(executable).ok()?;
    let metadata = std::fs::metadata(&path).ok()?;
    Some(Fingerprint {
        modified: metadata.modified().ok(),
        size: metadata.len(),
        path,
    })
}

/// How the agent's start went (D2).
#[derive(Debug, PartialEq, Eq)]
enum Start {
    Reported,
    /// A folder-trust question and no report: the start says nothing about the version.
    TrustAsked,
    Silent,
    NotStarted(String),
}

impl Host {
    fn checks_file(&self) -> PathBuf {
        self.paths.dir.join("agent-checks.json")
    }

    /// The recorded checks, read from the file the first time.
    fn recorded_checks(&self) -> Vec<AgentCheck> {
        let mut book = lock(&self.checks);
        if book.recorded.is_none() {
            book.recorded = Some(self.read_checks_file());
        }
        book.recorded.clone().unwrap_or_default()
    }

    fn read_checks_file(&self) -> Vec<AgentCheck> {
        let text = match std::fs::read_to_string(self.checks_file()) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
            Err(error) => {
                self.log
                    .line(&format!("agent checks not read error={error}"));
                return Vec::new();
            }
        };
        serde_json::from_str(&text).unwrap_or_else(|error| {
            self.log
                .line(&format!("agent checks not read error={error}"));
            Vec::new()
        })
    }

    /// Records `check` in place of the agent's earlier one.
    fn record_check(&self, check: &AgentCheck) {
        let mut checks = self.recorded_checks();
        checks.retain(|recorded| recorded.agent != check.agent);
        checks.push(check.clone());
        checks.sort_by_key(|recorded| recorded.agent.id());
        let file = self.checks_file();
        let written = serde_json::to_vec_pretty(&checks)
            .map_err(std::io::Error::other)
            .and_then(|bytes| {
                let partial = file.with_extension("json.partial");
                std::fs::write(&partial, bytes)?;
                std::fs::rename(&partial, &file)
            });
        if let Err(error) = written {
            self.log
                .line(&format!("agent checks not saved error={error}"));
        }
        lock(&self.checks).recorded = Some(checks);
    }

    /// The recorded check of `agent` when one of its checks failed (D3).
    pub(super) fn failed_check(&self, agent: Agent) -> Option<AgentCheck> {
        self.recorded_checks()
            .into_iter()
            .find(|check| check.agent == agent && check.failed())
    }

    /// Looks for new versions every interval, after waiting two (D1).
    pub(super) fn agent_check_loop(self: Arc<Self>) {
        thread::sleep(self.agent_check_interval * 2);
        loop {
            if self.login_path_settled.load(Ordering::SeqCst)
                && !self.stopping.load(Ordering::SeqCst)
            {
                self.check_new_versions();
            }
            thread::sleep(self.agent_check_interval);
        }
    }

    fn check_new_versions(&self) {
        for agent in Agent::ALL {
            if !self.agent_wanted(agent) {
                continue;
            }
            let Ok(executable) = self.resolve_agent(agent) else {
                continue;
            };
            let Some(print) = fingerprint(&executable) else {
                continue;
            };
            if lock(&self.checks).seen.insert(agent, print.clone()) == Some(print) {
                continue;
            }
            let Some(version) = self.version_of(agent, &executable) else {
                continue;
            };
            let checked = self
                .recorded_checks()
                .iter()
                .any(|check| check.agent == agent && check.version == version);
            if !checked {
                self.check_and_record(agent, &executable, version);
            }
        }
    }

    /// The agent's version; a check is tied to one, so an agent that does not say is
    /// skipped.
    fn version_of(&self, agent: Agent, executable: &Path) -> Option<String> {
        let version = hq_discovery::probe_version(executable, VERSION_TIMEOUT)
            .map(|answer| version_number(&answer));
        if version.is_none() {
            self.log.line(&format!(
                "agent check agent={} skipped reason=no version",
                agent.id()
            ));
        }
        version
    }

    /// `CheckAgents`: checks every agent turned on now whose version reads (D4).
    pub(super) fn check_agents_now(&self) -> Vec<AgentCheck> {
        let mut checks = Vec::new();
        for agent in Agent::ALL {
            if !self.agent_wanted(agent) {
                continue;
            }
            let Ok(executable) = self.resolve_agent(agent) else {
                continue;
            };
            if let Some(print) = fingerprint(&executable) {
                lock(&self.checks).seen.insert(agent, print);
            }
            let Some(version) = self.version_of(agent, &executable) else {
                continue;
            };
            checks.push(self.check_and_record(agent, &executable, version));
        }
        checks
    }

    fn check_and_record(&self, agent: Agent, executable: &Path, version: String) -> AgentCheck {
        let check = AgentCheck {
            agent,
            status: self.check_status(agent, executable),
            usage: self.check_usage(agent, executable),
            version,
        };
        self.log.line(&format!(
            "agent check agent={} version={} status={} usage={}",
            agent.id(),
            check.version,
            result_text(check.status),
            result_text(check.usage)
        ));
        self.record_check(&check);
        self.broadcast_usage();
        check
    }

    fn check_status(&self, agent: Agent, executable: &Path) -> CheckResult {
        let (folder, args): (Result<PathBuf, String>, &[&str]) = match agent {
            Agent::Claude => (
                self.trusted_claude_folder(),
                &["--setting-sources", "project,local"],
            ),
            Agent::Codex => (self.recent_folder(Agent::Codex), &[]),
            Agent::Agy => (self.agy_workspace(), &[]),
        };
        let folder = match folder {
            Ok(folder) => folder,
            Err(reason) => return self.not_checked(agent, "status", &reason),
        };
        match reports_at_start(executable, args, &folder) {
            Start::Reported => CheckResult::Ok,
            Start::TrustAsked => {
                self.not_checked(agent, "status", "the folder's trust question was shown")
            }
            Start::Silent => {
                self.log.line(&format!(
                    "agent check agent={} status failed reason=no status report within {} s",
                    agent.id(),
                    START_WAIT.as_secs()
                ));
                CheckResult::Failed
            }
            Start::NotStarted(reason) => {
                self.log.line(&format!(
                    "agent check agent={} status failed reason={reason}",
                    agent.id()
                ));
                CheckResult::Failed
            }
        }
    }

    fn check_usage(&self, agent: Agent, executable: &Path) -> CheckResult {
        let now = now_ms() / 1000;
        let outcome: Result<usize, String> = match agent {
            Agent::Claude => {
                let folder = match self.trusted_claude_folder() {
                    Ok(folder) => folder,
                    Err(reason) => return self.not_checked(agent, "usage", &reason),
                };
                match claude_usage::read_usage_screen(executable, &folder) {
                    Err(reason) if reason == claude_usage::TRUST_ASKED => {
                        return self.not_checked(agent, "usage", &reason);
                    }
                    outcome => outcome.map(|windows| windows.len()),
                }
            }
            Agent::Codex => windows_read(quota::read_codex(executable, CODEX_TIMEOUT, now).0),
            Agent::Agy => match self.agy_workspace() {
                Ok(folder) => windows_read(quota::read_agy(executable, &folder, AGY_TIMEOUT, now)),
                Err(reason) => return self.not_checked(agent, "usage", &reason),
            },
        };
        match outcome {
            Ok(windows) if windows > 0 => CheckResult::Ok,
            Ok(_) => {
                self.log.line(&format!(
                    "agent check agent={} usage failed reason=no usage window",
                    agent.id()
                ));
                CheckResult::Failed
            }
            Err(reason) => {
                self.log.line(&format!(
                    "agent check agent={} usage failed reason={reason}",
                    agent.id()
                ));
                CheckResult::Failed
            }
        }
    }

    fn not_checked(&self, agent: Agent, what: &str, reason: &str) -> CheckResult {
        self.log.line(&format!(
            "agent check agent={} {what} not-checked reason={reason}",
            agent.id()
        ));
        CheckResult::NotChecked
    }

    /// The folder of `agent`'s most recently active session that still exists.
    fn recent_folder(&self, agent: Agent) -> Result<PathBuf, String> {
        let rows = lock(&self.store)
            .list()
            .map_err(|error| format!("cannot list sessions: {error}"))?;
        rows.into_iter()
            .filter(|row| row.agent == agent)
            .max_by_key(|row| row.last_activity_at)
            .map(|row| PathBuf::from(row.cwd))
            .filter(|folder| folder.is_dir())
            .ok_or_else(|| format!("no folder {} was used in is known yet", agent.id()))
    }

    fn agy_workspace(&self) -> Result<PathBuf, String> {
        let folder = self.agy_folder();
        std::fs::create_dir_all(&folder)
            .map(|()| folder.clone())
            .map_err(|error| format!("cannot make {}: {error}", folder.display()))
    }
}

/// The version number in an agent's `--version` answer ("2.1.317 (Claude Code)",
/// "codex-cli 0.166.0"); the whole answer when none is found.
fn version_number(answer: &str) -> String {
    answer
        .split_whitespace()
        .map(|word| word.trim_start_matches('v'))
        .find(|word| word.starts_with(|first: char| first.is_ascii_digit()) && word.contains('.'))
        .unwrap_or(answer)
        .to_owned()
}

fn windows_read(report: QuotaReport) -> Result<usize, String> {
    match report {
        QuotaReport::Available(reading) => Ok(reading.windows.len()),
        QuotaReport::Unavailable { reason, .. } => Err(reason),
    }
}

fn result_text(result: CheckResult) -> &'static str {
    match result {
        CheckResult::Ok => "ok",
        CheckResult::Failed => "failed",
        CheckResult::NotChecked => "not-checked",
    }
}

/// Starts `program` in a terminal no one sees, answers its terminal's questions, and
/// waits for a root OSC 7501 report in any state; then hangs it up (D2).
fn reports_at_start(program: &Path, args: &[&str], folder: &Path) -> Start {
    let pair = match native_pty_system().openpty(PtySize {
        rows: ROWS,
        cols: COLUMNS,
        pixel_width: 0,
        pixel_height: 0,
    }) {
        Ok(pair) => pair,
        Err(error) => return Start::NotStarted(format!("cannot open a pseudo-terminal: {error}")),
    };
    let mut command = CommandBuilder::new(program);
    command.args(args);
    command.cwd(folder);
    for marker in adapters::PARENT_SESSION_MARKERS {
        command.env_remove(marker);
    }
    command.env("TERM", "xterm-256color");
    command.env("PATH", hq_discovery::shell::agent_path(program));
    let mut child = match pair.slave.spawn_command(command) {
        Ok(child) => child,
        Err(error) => return Start::NotStarted(format!("cannot start: {error}")),
    };
    drop(pair.slave);
    let start = match (pair.master.try_clone_reader(), pair.master.take_writer()) {
        (Ok(reader), Ok(writer)) => watch_start(reader, writer),
        (Err(error), _) | (_, Err(error)) => Start::NotStarted(format!("pty: {error}")),
    };
    let _ = child.kill();
    let deadline = Instant::now() + EXIT_WAIT;
    while Instant::now() < deadline {
        if matches!(child.try_wait(), Ok(Some(_))) {
            return start;
        }
        thread::sleep(Duration::from_millis(50));
    }
    if let Some(pid) = child.process_id() {
        external::force_kill(pid);
    }
    let _ = child.wait();
    start
}

fn watch_start(mut reader: Box<dyn Read + Send>, mut writer: Box<dyn Write + Send>) -> Start {
    let (chunks_tx, chunks) = mpsc::channel::<Vec<u8>>();
    // Reads until the terminal closes, so the agent can finish exiting.
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
    let mut deadline = Instant::now() + START_WAIT;
    let mut trust_asked = false;
    loop {
        let wait = deadline.saturating_duration_since(Instant::now());
        let Ok(chunk) = chunks.recv_timeout(wait) else {
            return if trust_asked {
                Start::TrustAsked
            } else {
                Start::Silent
            };
        };
        for signal in scanner.feed(&chunk) {
            match signal {
                TerminalSignal::ProgramStatusQuery => {
                    let _ = writer.write_all(PROGRAM_STATUS_SUPPORT_REPLY);
                }
                TerminalSignal::ProgramStatus(status) if status.id.is_none() => {
                    return Start::Reported;
                }
                _ => {}
            }
        }
        let _ = writer.write_all(&screen.feed(&chunk));
        let _ = writer.flush();
        if !trust_asked && screen.text().to_lowercase().contains("trust") {
            trust_asked = true;
            deadline = deadline.min(Instant::now() + AFTER_TRUST_WAIT);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn script(name: &str, body: &str) -> (PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!("hq-check-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("agent");
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        (dir, path)
    }

    #[test]
    fn a_root_report_at_start_passes() {
        let (dir, agent) = script(
            "reports",
            r"printf '\033]7501;state=idle:app=stand-in\007'; sleep 30",
        );
        assert_eq!(reports_at_start(&agent, &[], &dir), Start::Reported);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_report_while_asking_about_trust_still_passes() {
        let (dir, agent) = script(
            "agytrust",
            r"printf 'Trust this folder?\r\n'; sleep 0.2; printf '\033]7501;state=blocked:kind=permission\007'; sleep 30",
        );
        assert_eq!(reports_at_start(&agent, &[], &dir), Start::Reported);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_child_report_does_not_count() {
        let (dir, agent) = script(
            "child",
            r"printf '\033]7501;state=working:id=task-1\007'; printf 'Do you trust the files in this folder?\r\n'; sleep 30",
        );
        let started = Instant::now();
        assert_eq!(reports_at_start(&agent, &[], &dir), Start::TrustAsked);
        assert!(
            started.elapsed() < START_WAIT,
            "a trust question shortens the wait"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_agent_that_exits_without_reporting_fails() {
        let (dir, agent) = script("silent", "echo starting; exit 0");
        assert_eq!(reports_at_start(&agent, &[], &dir), Start::Silent);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_version_is_the_number_alone() {
        assert_eq!(version_number("2.1.317 (Claude Code)"), "2.1.317");
        assert_eq!(version_number("codex-cli 0.166.0"), "0.166.0");
        assert_eq!(version_number("1.4.12"), "1.4.12");
        assert_eq!(version_number("agy v1.5.0-beta.1"), "1.5.0-beta.1");
        assert_eq!(version_number("unknown build"), "unknown build");
    }

    #[test]
    fn a_new_version_changes_the_fingerprint() {
        let (dir, agent) = script("print", "exit 0");
        let before = fingerprint(&agent).unwrap();
        assert_eq!(fingerprint(&agent), Some(before.clone()), "unchanged");
        std::fs::write(&agent, "#!/bin/sh\necho newer build\n").unwrap();
        assert_ne!(fingerprint(&agent), Some(before), "rewritten");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
