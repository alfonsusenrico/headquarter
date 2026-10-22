//! Delivery of raw hook payloads from injected hook programs to a listener.
//!
//! The wire format is one JSON object per line:
//! `{"agent": "<id>", "raw": "<payload>", "session": "<host session id>"}`, where
//! `session` comes from the launching host's `HQ_SESSION` environment variable.
//! The payload stays a string so the hook side never has to understand it; the
//! listener normalizes it and drops all text. Payloads exist only in memory and on
//! this local socket.

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Upper bound for delivering one payload, keeping hooks far inside the one-second
/// budget even when the listener is stuck.
pub const SEND_TIMEOUT: Duration = Duration::from_millis(500);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ForwardedPayload {
    pub agent: String,
    pub raw: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

/// Sends one payload line. Errors are returned for tests; hook programs ignore them.
pub fn send(socket: &Path, agent: &str, raw: &str, session: Option<&str>) -> std::io::Result<()> {
    let mut stream = UnixStream::connect(socket)?;
    stream.set_write_timeout(Some(SEND_TIMEOUT))?;
    let mut line = serde_json::to_vec(&ForwardedPayload {
        agent: agent.to_owned(),
        raw: raw.to_owned(),
        session: session.map(str::to_owned),
    })?;
    line.push(b'\n');
    stream.write_all(&line)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufRead;
    use std::os::unix::net::UnixListener;

    fn temp_socket(name: &str) -> std::path::PathBuf {
        let path =
            std::env::temp_dir().join(format!("hq-forward-{}-{name}.sock", std::process::id()));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn missing_listener_fails_fast() {
        let socket = temp_socket("missing");
        let started = std::time::Instant::now();
        assert!(send(&socket, "claude", "{}", None).is_err());
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn payload_arrives_as_one_line() {
        let socket = temp_socket("present");
        let listener = UnixListener::bind(&socket).unwrap();
        send(&socket, "codex", "{\"a\":\n1}", Some("h1")).unwrap();
        let (stream, _) = listener.accept().unwrap();
        let line = std::io::BufReader::new(stream)
            .lines()
            .next()
            .unwrap()
            .unwrap();
        let payload: ForwardedPayload = serde_json::from_str(&line).unwrap();
        assert_eq!(payload.agent, "codex");
        assert_eq!(payload.raw, "{\"a\":\n1}");
        assert_eq!(payload.session.as_deref(), Some("h1"));
        let _ = std::fs::remove_file(&socket);
    }
}
