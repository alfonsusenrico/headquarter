//! Observe-only discovery of agent sessions started outside Headquarter.
//!
//! Nothing here signals, attaches to, or writes to another process or its files.
//! Session files are read only for metadata (event types, ids, paths, times); any
//! conversation text parsed along the way is dropped immediately.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value;
use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use crate::event::Agent;
use crate::paths;
use crate::status::Status;

/// Rollouts untouched for longer than this cannot belong to a live session.
const ROLLOUT_LOOKBACK: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// Enough of a rollout's tail to contain the latest turn events.
const ROLLOUT_TAIL_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExternalSession {
    pub agent: Agent,
    pub pid: Option<u32>,
    pub cwd: Option<String>,
    pub session_id: Option<String>,
    /// Unix seconds.
    pub started_at: Option<u64>,
    /// Unix seconds.
    pub last_activity_at: Option<u64>,
    /// `None` means unknown: no source supports a status.
    pub status: Option<Status>,
    /// Claude Code runs it in its own daemon (`kind: background`), not in a terminal: it
    /// keeps running after the program that started it ends (fix-claude-conversations).
    pub background: bool,
    /// The id `claude stop` and `claude attach` take for a background session (its `id`),
    /// which is not the conversation id.
    pub background_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceFailure {
    pub source: String,
    pub error: String,
}

#[derive(Debug, Default, Serialize)]
pub struct ScanReport {
    pub sessions: Vec<ExternalSession>,
    pub failures: Vec<SourceFailure>,
    /// What Claude Code says about the Claude agents the host runs, keyed by the
    /// excluded process each one runs under.
    pub hosted_claude: Vec<(u32, Status)>,
    /// Claude Code's background agents, when this scan asked `claude agents --json`;
    /// `None` when it did not, so the caller keeps the last list it had.
    pub claude_background: Option<Vec<ExternalSession>>,
}

/// What one scan keeps for the next, so a scan every few seconds stays cheap: the process
/// table, updated in place rather than read anew (a new table costs a system call per
/// process), and each Codex rollout's status, read again only when its file changed.
#[derive(Default)]
pub struct ScanMemory {
    system: System,
    rollouts: HashMap<PathBuf, (SystemTime, u64, Rollout)>,
}

/// `claude agents --json` normally answers in well under a second; a hung CLI must
/// not stall the scan.
const CLAUDE_AGENTS_TIMEOUT: Duration = Duration::from_secs(5);
/// `claude stop` answers at once; a hung CLI must not hold the host's request.
const CLAUDE_STOP_TIMEOUT: Duration = Duration::from_secs(10);

/// Scans every source; a failing source is reported and the others still run.
/// Processes in `exclude_pids`, and every process they started, are not outside
/// sessions (the host passes the agents it runs itself). `claude` is the Claude Code
/// executable, if one was found.
///
/// Claude Code's running sessions come from its own session records, which cost a few
/// file reads. Starting `claude agents --json` costs about 0.2 s of CPU and 160 MB, so
/// it runs only when `ask_claude` is set (the caller paces it, for background agents,
/// which have no record) or when this Claude Code keeps no records.
pub fn scan(
    memory: &mut ScanMemory,
    exclude_pids: &[u32],
    claude: Option<&Path>,
    ask_claude: bool,
) -> ScanReport {
    let mut report = ScanReport::default();
    let records =
        paths::claude_config_dir().and_then(|dir| claude_session_records(&dir.join("sessions")));
    let record_pids: Vec<u32> = records
        .iter()
        .flatten()
        .filter_map(|session| session.pid)
        .collect();
    refresh_process_table(&mut memory.system, &record_pids);
    let system = &memory.system;
    let excluded = |pid: u32| root_of(system, pid, exclude_pids).is_some();
    let records = records.map(|found| running_claude(found, system));
    let mut claude_found = records.clone().unwrap_or_default();
    if ask_claude || records.is_none() {
        let listed = match claude {
            Some(claude) => claude_sessions(claude, CLAUDE_AGENTS_TIMEOUT),
            None => Err("Claude Code was not found".to_owned()),
        };
        match listed {
            Ok(listed) => {
                let (background, running): (Vec<_>, Vec<_>) = listed
                    .into_iter()
                    .partition(|session| session.pid.is_none());
                if records.is_none() {
                    claude_found.extend(running);
                }
                claude_found.extend(background.iter().cloned());
                report.claude_background = Some(background);
            }
            Err(error) => report.failures.push(SourceFailure {
                source: "claude agents --json".to_owned(),
                error,
            }),
        }
    }
    for session in claude_found {
        let hosted = session
            .pid
            .and_then(|pid| root_of(system, pid, exclude_pids));
        match (hosted, session.status) {
            (Some(root), Some(status)) => report.hosted_claude.push((root, status)),
            (Some(_), None) => {}
            (None, _) => report.sessions.push(session),
        }
    }

    let processes: Vec<AgentProcess> = agent_processes(system, &[Agent::Codex, Agent::Agy])
        .into_iter()
        .filter(|process| !excluded(process.pid))
        .collect();

    let codex: Vec<&AgentProcess> = processes
        .iter()
        .filter(|process| process.agent == Agent::Codex)
        .collect();
    if !codex.is_empty() {
        let rollouts = match paths::codex_home() {
            Some(home) => codex_rollouts(&home.join("sessions"), &mut memory.rollouts),
            None => {
                report.failures.push(SourceFailure {
                    source: "codex sessions".to_owned(),
                    error: "home directory unknown".to_owned(),
                });
                Vec::new()
            }
        };
        report.sessions.extend(
            codex
                .iter()
                .map(|process| codex_session(process, &rollouts)),
        );
    }

    let agy: Vec<&AgentProcess> = processes
        .iter()
        .filter(|process| process.agent == Agent::Agy)
        .collect();
    // The conversation store is shared by every agy process, so its newest change can
    // only be attributed when a single agy process is running.
    let agy_activity = if agy.len() == 1 {
        paths::agy_dir().and_then(|dir| newest_mtime(&dir.join("conversations")))
    } else {
        None
    };
    report
        .sessions
        .extend(agy.iter().map(|process| ExternalSession {
            agent: Agent::Agy,
            pid: Some(process.pid),
            cwd: process.cwd.clone(),
            session_id: process.conversation.clone(),
            started_at: Some(process.started_at),
            last_activity_at: agy_activity,
            status: None,
            background: false,
            background_id: None,
        }));

    report
}

/// The conversations Claude Code runs now, asked directly (`claude agents --json`), for a
/// check that cannot wait for the next scan.
pub fn claude_running(claude: &Path) -> Result<Vec<ExternalSession>, String> {
    claude_sessions(claude, CLAUDE_AGENTS_TIMEOUT)
}

/// Stops a session in Claude Code's background: `claude stop <background id>`. The id is the
/// list's `id`, not the conversation id, which `claude stop` does not match.
pub fn claude_stop(claude: &Path, background_id: &str) -> Result<(), String> {
    let mut child = Command::new(claude)
        .args(["stop", background_id])
        .env("PATH", hq_discovery::shell::agent_path(claude))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot run claude: {error}"))?;
    let deadline = std::time::Instant::now() + CLAUDE_STOP_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(()),
            Ok(Some(status)) => return Err(format!("claude stop exited with {status}")),
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "claude stop did not finish within {} s",
                    CLAUDE_STOP_TIMEOUT.as_secs()
                ));
            }
        }
    }
}

fn claude_sessions(claude: &Path, timeout: Duration) -> Result<Vec<ExternalSession>, String> {
    let mut child = Command::new(claude)
        .args(["agents", "--json"])
        .env("PATH", hq_discovery::shell::agent_path(claude))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("cannot run claude: {error}"))?;
    let mut stdout = child.stdout.take().ok_or("no output pipe")?;
    let reader = std::thread::spawn(move || {
        let mut output = Vec::new();
        let _ = stdout.read_to_end(&mut output);
        output
    });
    let deadline = std::time::Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(Duration::from_millis(25));
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "claude agents did not answer within {} ms",
                    timeout.as_millis()
                ));
            }
        }
    };
    if !status.success() {
        return Err(format!("claude agents exited with {status}"));
    }
    let output = reader.join().map_err(|_| "reading claude output failed")?;
    parse_claude_agents(&String::from_utf8_lossy(&output))
}

/// Parses `claude agents --json`. `startedAt` is in milliseconds.
pub fn parse_claude_agents(json: &str) -> Result<Vec<ExternalSession>, String> {
    let value: Value = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let entries = value.as_array().ok_or("expected a JSON array")?;
    Ok(entries.iter().map(claude_entry).collect())
}

/// One Claude Code session, as `claude agents --json` lists it and as its session
/// record stores it (the same fields, seen in 2.1.303 to 2.1.310). "shell" (seen in
/// 2.1.310) means its turn ended while background shell commands still run: work for
/// the session, so working (fix-claude-shell-status).
fn claude_entry(entry: &Value) -> ExternalSession {
    let text = |key: &str| entry.get(key).and_then(Value::as_str).map(str::to_owned);
    let status = match (text("status").as_deref(), text("state").as_deref()) {
        (Some("busy" | "shell"), _) | (_, Some("working")) => Some(Status::Working),
        (Some("waiting"), _) | (_, Some("blocked")) => Some(Status::NeedsYou),
        (Some("idle"), _) | (_, Some("done" | "failed")) => Some(Status::Done),
        (_, Some("stopped")) => Some(Status::Inactive),
        _ => None,
    };
    ExternalSession {
        agent: Agent::Claude,
        pid: entry
            .get("pid")
            .and_then(Value::as_u64)
            .and_then(|pid| u32::try_from(pid).ok()),
        cwd: text("cwd"),
        session_id: text("sessionId"),
        started_at: entry
            .get("startedAt")
            .and_then(Value::as_u64)
            .map(|ms| ms / 1000),
        last_activity_at: None,
        status,
        background: text("kind").as_deref() == Some("background"),
        background_id: text("id").filter(|_| text("kind").as_deref() == Some("background")),
    }
}

/// Claude Code's records of its running sessions: `<pid>.json` in `dir`, one per
/// process, rewritten as its status changes. A record caught mid-write is skipped.
/// `None` when `dir` cannot be read, which older versions without records give.
///
/// Only `*.json` is opened: the same folder holds per-process key files.
fn claude_session_records(dir: &Path) -> Option<Vec<ExternalSession>> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut sessions: Vec<ExternalSession> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "json")
        })
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .filter_map(|text| serde_json::from_str::<Value>(&text).ok())
        .map(|record| claude_entry(&record))
        .collect();
    sessions.sort_by_key(|session| session.pid);
    Some(sessions)
}

/// The records whose process still runs Claude Code: one that has ended, or whose pid
/// now belongs to something else, left a stale record behind.
fn running_claude(records: Vec<ExternalSession>, system: &System) -> Vec<ExternalSession> {
    records
        .into_iter()
        .filter(|session| {
            session
                .pid
                .is_some_and(|pid| is_claude_process(system, pid))
        })
        .collect()
}

/// Whether `pid` runs Claude Code: its own executable, or a runtime started with it.
fn is_claude_process(system: &System, pid: u32) -> bool {
    let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) else {
        return false;
    };
    process.name() == "claude"
        || process
            .cmd()
            .iter()
            .any(|arg| arg.to_string_lossy().contains("claude"))
}

/// Whether any of Claude Code's background agents changed after `since`, read from the
/// modification times of their state files only (the files themselves hold settings
/// that must not be read).
pub fn claude_background_changed_since(since: SystemTime) -> bool {
    let Some(jobs) = paths::claude_config_dir().map(|dir| dir.join("jobs")) else {
        return false;
    };
    let Ok(entries) = std::fs::read_dir(jobs) else {
        return false;
    };
    entries.flatten().any(|entry| {
        std::fs::metadata(entry.path().join("state.json"))
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| modified > since)
    })
}

#[derive(Debug, Clone)]
struct AgentProcess {
    agent: Agent,
    pid: u32,
    cwd: Option<String>,
    started_at: u64,
    /// The conversation named on the command line (`agy --conversation <id>`).
    conversation: Option<String>,
}

/// Kills a process the host started that ignored being hung up. Only ever called with
/// the pid of an agent the host runs itself.
pub fn force_kill(pid: u32) -> bool {
    let pid = sysinfo::Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
    system
        .process(pid)
        .and_then(|process| process.kill_with(sysinfo::Signal::Kill))
        .unwrap_or(false)
}

/// Updates every process's name, parent and start time, which are cheap to read, and the
/// folder, command line and executable only of processes that may be agents: those
/// named after one, and `also`. Reading those for every process cost the host about
/// 0.6% of a core around the clock (perf/resource-use, 2026-10-21).
fn refresh_process_table(system: &mut System, also: &[u32]) {
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let candidates: Vec<sysinfo::Pid> = system
        .processes()
        .values()
        .filter(|process| {
            let name = process.name().to_string_lossy();
            Agent::ALL.iter().any(|agent| name == agent.binary())
                || also.contains(&process.pid().as_u32())
        })
        .map(sysinfo::Process::pid)
        .collect();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&candidates),
        false,
        ProcessRefreshKind::nothing()
            .with_cwd(UpdateKind::Always)
            .with_cmd(UpdateKind::Always)
            .with_exe(UpdateKind::Always),
    );
}

/// Whether `pid` is one of `roots` or started (directly or not) by one of them.
/// The root in `roots` that `pid` is or descends from.
fn root_of(system: &System, pid: u32, roots: &[u32]) -> Option<u32> {
    let mut current = Some(sysinfo::Pid::from_u32(pid));
    // Bounded walk: process trees are shallow, and a cycle must not hang the scan.
    for _ in 0..32 {
        let pid = current?;
        if roots.contains(&pid.as_u32()) {
            return Some(pid.as_u32());
        }
        current = system.process(pid).and_then(sysinfo::Process::parent);
    }
    None
}

fn agent_processes(system: &System, agents: &[Agent]) -> Vec<AgentProcess> {
    let mut found: Vec<AgentProcess> = system
        .processes()
        .values()
        .filter(|process| process.thread_kind().is_none())
        .filter_map(|process| {
            let pid = process.pid().as_u32();
            let name = process.name().to_string_lossy();
            let agent = agents
                .iter()
                .copied()
                .find(|agent| name == agent.binary())?;
            let args: Vec<String> = process
                .cmd()
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect();
            if is_background_service(agent, &args, process.exe()) {
                return None;
            }
            Some(AgentProcess {
                agent,
                pid,
                cwd: process.cwd().map(|cwd| cwd.to_string_lossy().into_owned()),
                started_at: process.start_time(),
                conversation: (agent == Agent::Agy)
                    .then(|| named_conversation(&args))
                    .flatten(),
            })
        })
        .collect();
    found.sort_by_key(|process| process.pid);
    found
}

/// The value of `--conversation <id>` or `--conversation=<id>`, which agy resumes.
fn named_conversation(args: &[String]) -> Option<String> {
    args.iter().enumerate().find_map(|(index, arg)| {
        match arg.strip_prefix("--conversation") {
            Some("") => args.get(index + 1).cloned(),
            Some(rest) => rest.strip_prefix('=').map(str::to_owned),
            None => None,
        }
        .filter(|id| !id.is_empty() && !id.starts_with('-'))
    })
}

/// Codex runs background services under the same executable name; they are not
/// sessions.
fn is_background_service(agent: Agent, args: &[String], exe: Option<&Path>) -> bool {
    agent == Agent::Codex
        && (args
            .iter()
            .any(|arg| arg == "app-server" || arg == "mcp-server")
            || exe.is_some_and(|exe| exe.to_string_lossy().contains("app-server-daemon")))
}

/// The Codex conversation (thread) id of a session started in `cwd` at
/// `started_at_secs`, matched by folder and time from Codex's rollout files.
pub fn codex_conversation(cwd: &str, started_at_secs: u64) -> Option<String> {
    let rollouts = codex_rollouts(&paths::codex_home()?.join("sessions"), &mut HashMap::new());
    let process = AgentProcess {
        agent: Agent::Codex,
        pid: 0,
        cwd: Some(cwd.to_owned()),
        started_at: started_at_secs,
        conversation: None,
    };
    codex_session(&process, &rollouts).session_id
}

/// The agy conversation created at or after `started_at_secs`, when exactly one was.
/// agy keeps one store file per conversation, named by its id.
pub fn agy_conversation_since(started_at_secs: u64) -> Option<String> {
    let dir = paths::agy_dir()?.join("conversations");
    let created: Vec<String> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let born = metadata.created().or_else(|_| metadata.modified()).ok()?;
            let path = entry.path();
            (unix_secs(born) >= started_at_secs && path.extension()? == "db")
                .then(|| path.file_stem()?.to_str().map(str::to_owned))
                .flatten()
        })
        .collect();
    match created.as_slice() {
        [only] => Some(only.clone()),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Rollout {
    cwd: String,
    session_id: Option<String>,
    modified: u64,
    status: Option<Status>,
}

fn codex_session(process: &AgentProcess, rollouts: &[Rollout]) -> ExternalSession {
    let rollout = rollouts
        .iter()
        .filter(|rollout| Some(&rollout.cwd) == process.cwd.as_ref())
        .filter(|rollout| rollout.modified + 60 >= process.started_at)
        .max_by_key(|rollout| rollout.modified);
    ExternalSession {
        agent: Agent::Codex,
        pid: Some(process.pid),
        cwd: process.cwd.clone(),
        session_id: rollout.and_then(|rollout| rollout.session_id.clone()),
        started_at: Some(process.started_at),
        last_activity_at: rollout.map(|rollout| rollout.modified),
        status: rollout.and_then(|rollout| rollout.status),
        background: false,
        background_id: None,
    }
}

/// The recent rollouts, each read again only when its modification time or length
/// changed since `known` last saw it; files gone or too old leave `known`.
fn codex_rollouts(
    sessions_dir: &Path,
    known: &mut HashMap<PathBuf, (SystemTime, u64, Rollout)>,
) -> Vec<Rollout> {
    let cutoff = SystemTime::now()
        .checked_sub(ROLLOUT_LOOKBACK)
        .unwrap_or(UNIX_EPOCH);
    let mut files = Vec::new();
    collect_files(sessions_dir, &mut files, &|path| {
        path.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("rollout-"))
            && path.extension().is_some_and(|ext| ext == "jsonl")
    });
    let mut current = HashMap::new();
    let rollouts = files
        .into_iter()
        .filter_map(|path| {
            let metadata = std::fs::metadata(&path).ok()?;
            let modified = metadata.modified().ok()?;
            if modified < cutoff {
                return None;
            }
            let rollout = match known.remove(&path) {
                Some((seen, length, rollout)) if seen == modified && length == metadata.len() => {
                    rollout
                }
                _ => {
                    let (cwd, session_id) = rollout_meta(&path)?;
                    Rollout {
                        cwd,
                        session_id,
                        modified: unix_secs(modified),
                        status: rollout_status(&rollout_tail(&path).ok()?),
                    }
                }
            };
            current.insert(path, (modified, metadata.len(), rollout.clone()));
            Some(rollout)
        })
        .collect();
    *known = current;
    rollouts
}

pub(crate) fn collect_files(dir: &Path, out: &mut Vec<PathBuf>, keep: &dyn Fn(&Path) -> bool) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => collect_files(&path, out, keep),
            Ok(kind) if kind.is_file() && keep(&path) => out.push(path),
            _ => {}
        }
    }
}

/// The working folder and session id from a rollout's first `session_meta` line.
fn rollout_meta(path: &Path) -> Option<(String, Option<String>)> {
    let mut first = String::new();
    BufReader::new(File::open(path).ok()?)
        .read_line(&mut first)
        .ok()?;
    let line: Value = serde_json::from_str(&first).ok()?;
    if line.get("type").and_then(Value::as_str) != Some("session_meta") {
        return None;
    }
    let payload = line.get("payload")?;
    let cwd = payload.get("cwd")?.as_str()?.to_owned();
    let session_id = payload
        .get("session_id")
        .or_else(|| payload.get("id"))
        .and_then(Value::as_str)
        .map(str::to_owned);
    Some((cwd, session_id))
}

pub(crate) fn rollout_tail(path: &Path) -> std::io::Result<String> {
    let mut file = File::open(path)?;
    let length = file.metadata()?.len();
    let start = length.saturating_sub(ROLLOUT_TAIL_BYTES);
    file.seek(SeekFrom::Start(start))?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if start > 0 {
        // Drop the partial first line.
        text = text
            .split_once('\n')
            .map(|(_, rest)| rest.to_owned())
            .unwrap_or_default();
    }
    Ok(text)
}

/// Status from the latest turn event in a rollout, by event type only.
fn rollout_status(tail: &str) -> Option<Status> {
    tail.lines().rev().find_map(|line| {
        let value: Value = serde_json::from_str(line).ok()?;
        if value.get("type").and_then(Value::as_str) != Some("event_msg") {
            return None;
        }
        let kind = value.get("payload")?.get("type")?.as_str()?;
        if kind.contains("approval_request") || kind == "request_user_input" {
            Some(Status::NeedsYou)
        } else {
            match kind {
                "task_started" => Some(Status::Working),
                "task_complete" | "turn_aborted" => Some(Status::Done),
                _ => None,
            }
        }
    })
}

fn newest_mtime(dir: &Path) -> Option<u64> {
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| entry.metadata().ok()?.modified().ok())
        .max()
        .map(unix_secs)
}

fn unix_secs(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0)
}

/// Groups sessions by working folder, the sidebar's grouping rule.
pub fn by_folder(sessions: &[ExternalSession]) -> HashMap<String, Vec<&ExternalSession>> {
    let mut groups: HashMap<String, Vec<&ExternalSession>> = HashMap::new();
    for session in sessions {
        let folder = session
            .cwd
            .clone()
            .unwrap_or_else(|| "(unknown)".to_owned());
        groups.entry(folder).or_default().push(session);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_agents_statuses() {
        let json = r#"[
            {"pid": 10, "cwd": "/a", "kind": "interactive", "name": "x", "sessionId": "s1", "startedAt": 1792000000123, "status": "busy"},
            {"pid": 11, "cwd": "/b", "kind": "interactive", "sessionId": "s2", "status": "waiting", "waitingFor": "permission prompt"},
            {"pid": 12, "cwd": "/c", "kind": "interactive", "sessionId": "s3", "status": "idle"},
            {"cwd": "/d", "kind": "background", "id": "b1", "state": "stopped"},
            {"cwd": "/e", "kind": "interactive", "status": "something-new"},
            {"pid": 13, "cwd": "/f", "kind": "interactive", "sessionId": "s4", "status": "shell"}
        ]"#;
        let sessions = parse_claude_agents(json).unwrap();
        let statuses: Vec<Option<Status>> = sessions.iter().map(|s| s.status).collect();
        assert_eq!(
            statuses,
            vec![
                Some(Status::Working),
                Some(Status::NeedsYou),
                Some(Status::Done),
                Some(Status::Inactive),
                None,
                Some(Status::Working)
            ]
        );
        assert_eq!(sessions[0].started_at, Some(1792000000));
        assert_eq!(sessions[0].session_id.as_deref(), Some("s1"));
        let background: Vec<bool> = sessions.iter().map(|s| s.background).collect();
        assert_eq!(background, [false, false, false, true, false, false]);
    }

    #[test]
    fn claude_records_list_only_live_claude_processes() {
        let dir = std::env::temp_dir().join(format!("hq-claude-records-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(claude_session_records(&dir.join("missing")), None);

        // A process whose command names Claude Code stands in for a running session.
        let mut live = Command::new("/bin/sh")
            .args(["-c", "sleep 30; exit 0", "claude"])
            .spawn()
            .unwrap();
        let mut ended = Command::new("/bin/sh")
            .args(["-c", "exit 0", "claude"])
            .spawn()
            .unwrap();
        let ended_pid = ended.id();
        ended.wait().unwrap();
        let record = |pid: u32, status: &str| {
            format!(
                r#"{{"pid":{pid},"cwd":"/w","kind":"interactive","sessionId":"s{pid}","startedAt":1792000000123,"status":"{status}","waitingFor":"input needed","futureField":[1]}}"#
            )
        };
        std::fs::write(
            dir.join(format!("{}.json", live.id())),
            record(live.id(), "waiting"),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{ended_pid}.json")),
            record(ended_pid, "busy"),
        )
        .unwrap();
        std::fs::write(
            dir.join(format!("{}.abc.key", live.id())),
            record(live.id(), "busy"),
        )
        .unwrap();
        std::fs::write(dir.join("partial.json"), r#"{"pid":"#).unwrap();

        let records = claude_session_records(&dir).unwrap();
        assert_eq!(
            records.len(),
            2,
            "both records parse; the key file is not read"
        );
        let pids: Vec<u32> = records.iter().filter_map(|session| session.pid).collect();
        let mut system = System::new();
        refresh_process_table(&mut system, &pids);
        let sessions = running_claude(records, &system);
        let _ = live.kill();
        let _ = live.wait();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(sessions.len(), 1, "{sessions:?}");
        assert_eq!(sessions[0].pid, Some(live.id()));
        assert_eq!(sessions[0].status, Some(Status::NeedsYou));
        assert_eq!(sessions[0].started_at, Some(1792000000));
        assert_eq!(sessions[0].cwd.as_deref(), Some("/w"));
    }

    #[test]
    fn a_hung_claude_cli_times_out() {
        use std::os::unix::fs::PermissionsExt;
        let stub = std::env::temp_dir().join(format!("hq-hung-claude-{}", std::process::id()));
        std::fs::write(&stub, "#!/bin/sh\nsleep 10\n").unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let started = std::time::Instant::now();
        let result = claude_sessions(&stub, Duration::from_millis(1_500));
        assert!(result.unwrap_err().contains("did not answer"));
        assert!(started.elapsed() < Duration::from_secs(4));
        let _ = std::fs::remove_file(stub);
    }

    #[test]
    fn claude_agents_garbage_is_an_error() {
        assert!(parse_claude_agents("not json").is_err());
        assert!(parse_claude_agents("{}").is_err());
    }

    #[test]
    fn rollout_status_uses_latest_turn_event_only() {
        let tail = [
            r#"{"type":"event_msg","payload":{"type":"task_started"}}"#,
            r#"{"type":"response_item","payload":{"type":"message","content":"SECRET"}}"#,
            r#"{"type":"event_msg","payload":{"type":"token_count"}}"#,
        ]
        .join("\n");
        assert_eq!(rollout_status(&tail), Some(Status::Working));

        let done = format!(
            "{tail}\n{}",
            r#"{"type":"event_msg","payload":{"type":"task_complete","last_agent_message":"SECRET"}}"#
        );
        assert_eq!(rollout_status(&done), Some(Status::Done));

        let approval = format!(
            "{tail}\n{}",
            r#"{"type":"event_msg","payload":{"type":"exec_approval_request"}}"#
        );
        assert_eq!(rollout_status(&approval), Some(Status::NeedsYou));

        assert_eq!(rollout_status("garbage\n{}"), None);
    }

    #[test]
    fn codex_services_are_not_sessions() {
        let args = |list: &[&str]| list.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
        assert!(is_background_service(
            Agent::Codex,
            &args(&["codex", "app-server"]),
            None
        ));
        assert!(is_background_service(
            Agent::Codex,
            &args(&["codex"]),
            Some(Path::new("/x/app-server-daemon/bin/codex"))
        ));
        assert!(!is_background_service(
            Agent::Codex,
            &args(&["codex"]),
            None
        ));
        assert!(!is_background_service(
            Agent::Agy,
            &args(&["agy", "app-server"]),
            None
        ));
    }

    #[test]
    fn agy_names_its_conversation_on_the_command_line() {
        let args = |list: &[&str]| list.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            named_conversation(&args(&["agy", "--conversation", "conv-1"])).as_deref(),
            Some("conv-1")
        );
        assert_eq!(
            named_conversation(&args(&["agy", "--conversation=conv-1"])).as_deref(),
            Some("conv-1")
        );
        assert_eq!(named_conversation(&args(&["agy", "-c"])), None);
        assert_eq!(named_conversation(&args(&["agy", "--conversation"])), None);
        assert_eq!(
            named_conversation(&args(&["agy", "--conversation", "--mode"])),
            None
        );
    }

    #[test]
    fn codex_session_matches_rollout_by_folder_and_time() {
        let process = AgentProcess {
            agent: Agent::Codex,
            pid: 5,
            cwd: Some("/w".into()),
            started_at: 1_000,
            conversation: None,
        };
        let rollouts = vec![
            Rollout {
                cwd: "/w".into(),
                session_id: Some("old".into()),
                modified: 500,
                status: Some(Status::Done),
            },
            Rollout {
                cwd: "/w".into(),
                session_id: Some("new".into()),
                modified: 1_200,
                status: Some(Status::Working),
            },
            Rollout {
                cwd: "/other".into(),
                session_id: Some("x".into()),
                modified: 2_000,
                status: Some(Status::Done),
            },
        ];
        let session = codex_session(&process, &rollouts);
        assert_eq!(session.session_id.as_deref(), Some("new"));
        assert_eq!(session.status, Some(Status::Working));

        let unmatched = codex_session(
            &AgentProcess {
                cwd: Some("/nowhere".into()),
                ..process
            },
            &rollouts,
        );
        assert_eq!(unmatched.status, None);
    }

    #[test]
    fn rollouts_are_read_again_only_when_their_file_changes() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("hq-rollouts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rollout-a.jsonl");
        std::fs::write(
            &path,
            "{\"type\":\"session_meta\",\"payload\":{\"cwd\":\"/w\",\"session_id\":\"t1\"}}\n{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_started\"}}\n",
        )
        .unwrap();
        let mut known = HashMap::new();
        let first = codex_rollouts(&dir, &mut known);
        assert_eq!(first[0].status, Some(Status::Working));

        // Unchanged: the remembered result is used, not the file.
        known.get_mut(&path).unwrap().2.status = None;
        assert_eq!(codex_rollouts(&dir, &mut known)[0].status, None);

        // Changed: read again.
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        file.write_all(b"{\"type\":\"event_msg\",\"payload\":{\"type\":\"task_complete\"}}\n")
            .unwrap();
        assert_eq!(
            codex_rollouts(&dir, &mut known)[0].status,
            Some(Status::Done)
        );

        // Gone: forgotten.
        std::fs::remove_file(&path).unwrap();
        assert!(codex_rollouts(&dir, &mut known).is_empty());
        assert!(known.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rollout_meta_reads_first_line() {
        let path = std::env::temp_dir().join(format!("rollout-test-{}.jsonl", std::process::id()));
        std::fs::write(
            &path,
            "{\"type\":\"session_meta\",\"payload\":{\"cwd\":\"/w\",\"session_id\":\"t1\",\"base_instructions\":\"SECRET\"}}\n{}\n",
        )
        .unwrap();
        assert_eq!(
            rollout_meta(&path),
            Some(("/w".to_owned(), Some("t1".to_owned())))
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn grouping_by_folder() {
        let session = |cwd: Option<&str>| ExternalSession {
            agent: Agent::Codex,
            pid: None,
            cwd: cwd.map(str::to_owned),
            session_id: None,
            started_at: None,
            last_activity_at: None,
            status: None,
            background: false,
            background_id: None,
        };
        let sessions = vec![session(Some("/a")), session(Some("/a")), session(None)];
        let groups = by_folder(&sessions);
        assert_eq!(groups["/a"].len(), 2);
        assert_eq!(groups["(unknown)"].len(), 1);
    }
}
