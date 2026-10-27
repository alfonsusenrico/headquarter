//! Programs that agents run on a host's behalf, shared by `hq` and `hq-probe`.
//! They must never change the agent's behaviour: `hook_forward` prints nothing and
//! always succeeds, and `hook_status_line` prints exactly what the owner's own status
//! line prints.

use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::event::Agent;
use crate::quota::{self, QuotaReading};

/// Environment variable carrying the host's session id into the agent and its hooks.
pub const SESSION_ENV: &str = "HQ_SESSION";

pub fn hook_forward(socket: &Path, agent: &str, payload: Option<String>) {
    let raw = match payload {
        Some(payload) => payload,
        None => read_stdin_text(),
    };
    let session = std::env::var(SESSION_ENV).ok();
    // Delivery failure (host gone, socket stale) is expected and ignored.
    let _ = crate::forward::send(socket, agent, &raw, session.as_deref());
}

fn read_stdin_text() -> String {
    let mut stdin = std::io::stdin();
    if stdin.is_terminal() {
        return String::new();
    }
    let mut raw = String::new();
    let _ = stdin.read_to_string(&mut raw);
    raw
}

/// The forwarded-payload "agent" name for session details, which the host keeps
/// apart from hook events.
pub const DETAILS_KIND: &str = "details";

/// Records Claude rate limits from status line input, forwards the session's details to
/// the host at `socket` (numbers and names only, `crate::details`), then runs the
/// owner's own status line command with the same input. Returns the process exit code.
pub fn hook_status_line(chain: Option<&str>, socket: Option<&Path>) -> u8 {
    let mut input = Vec::new();
    let _ = std::io::stdin().read_to_end(&mut input);
    record_claude_limits(&input);
    if let Some(socket) = socket {
        forward_details(socket, &input);
    }

    let Some(command) = chain else {
        return 0;
    };
    let Ok(mut child) = Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdin(Stdio::piped())
        .spawn()
    else {
        return 1;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&input);
    }
    match child.wait() {
        Ok(status) => u8::try_from(status.code().unwrap_or(1)).unwrap_or(1),
        Err(_) => 1,
    }
}

fn forward_details(socket: &Path, input: &[u8]) {
    let Ok(value) = serde_json::from_slice(input) else {
        return;
    };
    let details = crate::details::from_claude_status_line(&value);
    let Ok(serde_json::Value::Object(mut report)) = serde_json::to_value(&details) else {
        return;
    };
    // The conversation lets the host tell this session's figures from those of another
    // Claude Code conversation started with the same environment (fix-hook-session-match).
    if let Some(conversation) = value.get("session_id").and_then(serde_json::Value::as_str) {
        report.insert("session_id".to_owned(), conversation.into());
    }
    let Ok(raw) = serde_json::to_string(&report) else {
        return;
    };
    let session = std::env::var(SESSION_ENV).ok();
    // Delivery failure (host gone, socket stale) is expected and ignored.
    let _ = crate::forward::send(socket, DETAILS_KIND, &raw, session.as_deref());
}

fn record_claude_limits(input: &[u8]) {
    let Ok(value) = serde_json::from_slice(input) else {
        return;
    };
    let Some(windows) = quota::parse_claude_status_line(&value) else {
        return;
    };
    let Some(path) = claude_snapshot_path() else {
        return;
    };
    let reading = QuotaReading {
        provider: Agent::Claude,
        windows,
        read_at: crate::now_ms() / 1000,
    };
    let _ = quota::save_snapshot(&path, &reading);
}

pub fn claude_snapshot_path() -> Option<PathBuf> {
    crate::paths::quota_dir().map(|dir| dir.join("claude.json"))
}

/// Where the Phase 0 tool kept the Claude snapshot; read once by the host if the
/// current file is missing.
pub fn legacy_claude_snapshot_path() -> Option<PathBuf> {
    crate::paths::probe_dir().map(|dir| dir.join("quota-claude.json"))
}

/// Replies the owner's terminal sends to an agent's queries (cursor position, device
/// attributes, status, focus, OSC colour reports) arrive as input but are not the
/// owner interacting.
pub fn is_terminal_report(input: &[u8]) -> bool {
    match input {
        [0x1b, b']', ..] => true,
        [0x1b, b'[', body @ .., last] => {
            matches!(last, b'R' | b'c' | b'n' | b'I' | b'O')
                && body
                    .iter()
                    .all(|byte| byte.is_ascii_digit() || b";?>".contains(byte))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_terminal_report;

    #[test]
    fn terminal_reports_are_not_owner_input() {
        for report in [
            &b"\x1b[12;40R"[..],
            b"\x1b[?62;22c",
            b"\x1b[0n",
            b"\x1b[I",
            b"\x1b]11;rgb:0000/0000/0000\x07",
        ] {
            assert!(is_terminal_report(report), "{report:?}");
        }
        for key in [
            &b"x"[..],
            b"\r",
            b"\x1b[A",
            b"\x1b[5~",
            b"\x7f",
            b"\x03",
            b"\x1b",
        ] {
            assert!(!is_terminal_report(key), "{key:?}");
        }
    }
}
