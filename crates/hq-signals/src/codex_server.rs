//! Codex's shared background server, where Codex runs its threads, sub-agents and
//! background commands for every Codex window. `codex app-server proxy` connects stdio
//! to the server's control socket, which speaks WebSocket: an HTTP upgrade, then
//! JSON-RPC in text frames. Used to stop sub-agents still running for a session being
//! closed.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use crate::subagents::RunningSubagent;

const UPGRADE: &[u8] = b"GET / HTTP/1.1\r\nHost: localhost\r\nUpgrade: websocket\r\n\
Connection: Upgrade\r\nSec-WebSocket-Key: aHEtY29kZXgtc2VydmVyLQ==\r\nSec-WebSocket-Version: 13\r\n\r\n";

/// Interrupts each sub-agent's running turn. Returns how many the server accepted.
pub fn interrupt_turns(
    executable: &Path,
    turns: &[RunningSubagent],
    timeout: Duration,
) -> Result<usize, String> {
    if turns.is_empty() {
        return Ok(0);
    }
    let mut command = Command::new(executable);
    command
        .args(["app-server", "proxy"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for marker in crate::adapters::PARENT_SESSION_MARKERS {
        command.env_remove(marker);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot start `codex app-server proxy`: {error}"))?;
    let result = exchange(&mut child, turns, timeout);
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn exchange(
    child: &mut std::process::Child,
    turns: &[RunningSubagent],
    timeout: Duration,
) -> Result<usize, String> {
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let mut stdout = child.stdout.take().ok_or("no stdout")?;
    let (bytes_tx, bytes_rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        let mut buffer = [0_u8; 16 * 1024];
        while let Ok(count) = stdout.read(&mut buffer) {
            if count == 0 || bytes_tx.send(buffer[..count].to_vec()).is_err() {
                break;
            }
        }
    });
    let write = |stdin: &mut std::process::ChildStdin, bytes: &[u8]| {
        stdin
            .write_all(bytes)
            .and_then(|()| stdin.flush())
            .map_err(|error| format!("codex proxy write: {error}"))
    };
    let deadline = Instant::now() + timeout;
    let mut received = Vec::new();
    let next = |received: &mut Vec<u8>| -> Result<(), String> {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let chunk = bytes_rx
            .recv_timeout(remaining)
            .map_err(|_| "Codex's background server did not answer in time".to_owned())?;
        received.extend(chunk);
        Ok(())
    };

    write(&mut stdin, UPGRADE)?;
    let header_end = loop {
        if let Some(end) = find(&received, b"\r\n\r\n") {
            break end;
        }
        next(&mut received)?;
    };
    let status_line = String::from_utf8_lossy(&received[..header_end]);
    if !status_line.starts_with("HTTP/1.1 101") {
        return Err("Codex's background server refused the connection".to_owned());
    }
    received.drain(..header_end + 4);

    let mask = mask_key();
    let mut messages = vec![
        json!({"id": 1, "method": "initialize",
               "params": {"clientInfo": {"name": "headquarter", "version": env!("CARGO_PKG_VERSION")}}}),
        json!({"method": "initialized"}),
    ];
    let first_interrupt = 2;
    for (index, turn) in turns.iter().enumerate() {
        messages.push(
            json!({"id": first_interrupt + index, "method": "turn/interrupt",
            "params": {"threadId": turn.thread_id, "turnId": turn.turn_id}}),
        );
    }
    for message in &messages {
        write(&mut stdin, &text_frame(&message.to_string(), mask))?;
    }

    let mut answered = 0;
    let mut accepted = 0;
    while answered < turns.len() {
        while let Some((payload, used)) = server_frame(&received) {
            received.drain(..used);
            let Ok(message) = serde_json::from_slice::<Value>(&payload) else {
                continue;
            };
            let id = message.get("id").and_then(Value::as_u64).unwrap_or(0);
            if id >= first_interrupt as u64 {
                answered += 1;
                if message.get("error").is_none() {
                    accepted += 1;
                }
            }
        }
        if answered < turns.len() {
            next(&mut received)?;
        }
    }
    Ok(accepted)
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Clients must mask their frames; the key only needs to vary between connections.
fn mask_key() -> [u8; 4] {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.subsec_nanos());
    (nanos ^ std::process::id().rotate_left(16) | 1).to_be_bytes()
}

/// A masked, final text frame.
fn text_frame(text: &str, mask: [u8; 4]) -> Vec<u8> {
    let data = text.as_bytes();
    let mut frame = vec![0x81];
    match data.len() {
        length @ 0..=125 => frame.push(0x80 | length as u8),
        length @ 126..=0xFFFF => {
            frame.push(0x80 | 126);
            frame.extend((length as u16).to_be_bytes());
        }
        length => {
            frame.push(0x80 | 127);
            frame.extend((length as u64).to_be_bytes());
        }
    }
    frame.extend(mask);
    frame.extend(
        data.iter()
            .enumerate()
            .map(|(index, byte)| byte ^ mask[index % 4]),
    );
    frame
}

/// The first complete frame from the server, as its payload and the bytes it used.
/// Control frames come back with their payload too; callers skip what is not JSON.
fn server_frame(bytes: &[u8]) -> Option<(Vec<u8>, usize)> {
    let second = *bytes.get(1)?;
    let masked = second & 0x80 != 0;
    let (length, mut offset) = match second & 0x7F {
        126 => (
            u16::from_be_bytes([*bytes.get(2)?, *bytes.get(3)?]) as usize,
            4,
        ),
        127 => {
            let raw: [u8; 8] = bytes.get(2..10)?.try_into().ok()?;
            (usize::try_from(u64::from_be_bytes(raw)).ok()?, 10)
        }
        length => (length as usize, 2),
    };
    let mask = if masked {
        let key: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
        offset += 4;
        Some(key)
    } else {
        None
    };
    let payload = bytes.get(offset..offset + length)?;
    let payload = match mask {
        Some(key) => payload
            .iter()
            .enumerate()
            .map(|(index, byte)| byte ^ key[index % 4])
            .collect(),
        None => payload.to_vec(),
    };
    Some((payload, offset + length))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Unmasks a client frame the way a server does.
    fn read_client(frame: &[u8]) -> String {
        let (payload, used) = server_frame(frame).unwrap();
        assert_eq!(used, frame.len());
        String::from_utf8(payload).unwrap()
    }

    #[test]
    fn text_frames_round_trip_at_every_length_form() {
        let mask = [1, 2, 3, 4];
        for length in [0, 5, 125, 126, 300, 70_000] {
            let text = "x".repeat(length);
            let frame = text_frame(&text, mask);
            assert_eq!(frame[0], 0x81, "final text frame");
            assert_ne!(frame[1] & 0x80, 0, "client frames are masked");
            assert_eq!(read_client(&frame), text);
        }
    }

    #[test]
    fn server_frames_are_read_whole_or_not_at_all() {
        let mut stream = vec![0x81, 2, b'{', b'}'];
        stream.extend([0x81, 126, 0, 3, b'a']);
        let (first, used) = server_frame(&stream).unwrap();
        assert_eq!((first.as_slice(), used), (b"{}".as_slice(), 4));
        assert_eq!(
            server_frame(&stream[used..]),
            None,
            "second frame incomplete"
        );
        assert_eq!(server_frame(&[0x81]), None);
    }

    #[test]
    fn nothing_to_interrupt_starts_nothing() {
        assert_eq!(
            interrupt_turns(Path::new("/nonexistent/codex"), &[], Duration::from_secs(1)),
            Ok(0)
        );
        let turn = RunningSubagent {
            thread_id: "t".into(),
            turn_id: "u".into(),
        };
        assert!(
            interrupt_turns(
                Path::new("/nonexistent/codex"),
                &[turn],
                Duration::from_secs(1)
            )
            .is_err()
        );
    }
}
