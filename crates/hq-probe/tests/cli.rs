//! End-to-end checks of the probe binary with stub agents; no real agent CLI runs.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::Value;

const PROBE: &str = env!("CARGO_BIN_EXE_hq-probe");

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hq-probe-test-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// A data home under a short path: Unix socket paths are limited to ~104 bytes.
fn short_data_home(name: &str) -> PathBuf {
    let dir = PathBuf::from("/tmp").join(format!("hqt-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn run_passes_output_through_and_records_status() {
    let dir = scratch("run");
    let data_home = short_data_home("run");
    let stub = dir.join("stub-agent.sh");
    std::fs::write(
        &stub,
        concat!(
            "#!/bin/sh\n",
            "printf '\\033]0;secret title\\007'\n",
            "printf 'hello\\n'\n",
            "printf '\\033]7501;state=working\\033\\\\'\n",
            "sleep 0.5\n",
            "printf '\\033]7501;state=blocked:kind=permission\\007'\n",
            "sleep 0.5\n",
            "printf '\\033]7501;state=done\\007'\n",
            "printf '\\007'\n",
            "sleep 0.5\n",
        ),
    )
    .unwrap();
    let log = dir.join("events.jsonl");

    let mut child = Command::new(PROBE)
        .args(["run", "agy", "--program", "sh", "--log"])
        .arg(&log)
        .arg("--")
        .arg(&stub)
        .env("XDG_DATA_HOME", &data_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // One keystroke, as the owner would type, before the stub starts its turn.
    child.stdin.take().unwrap().write_all(b"x").unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success(), "{output:?}");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("\x1b]0;secret title\x07"));
    assert!(stdout.contains("hello"));
    assert!(stdout.contains("\x1b]7501;state=blocked:kind=permission\x07"));

    let text = std::fs::read_to_string(&log).unwrap();
    assert!(
        !text.contains("secret title"),
        "title text must not be logged"
    );
    let records: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let statuses: Vec<&str> = records
        .iter()
        .filter(|record| record["type"] == "status")
        .map(|record| record["status"].as_str().unwrap())
        .collect();
    for expected in ["working", "needs-you", "done-unread", "inactive"] {
        assert!(
            statuses.contains(&expected),
            "missing {expected} in {statuses:?}"
        );
    }
    let signals: Vec<&str> = records
        .iter()
        .filter(|record| record["type"] == "signal")
        .map(|record| record["signal"].as_str().unwrap())
        .collect();
    assert!(signals.contains(&"title"));
    assert!(signals.contains(&"bell"));
    assert!(signals.contains(&"program-status:blocked:permission"));
    assert!(records.iter().any(|record| record["type"] == "exit"));

    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&data_home);
}

#[test]
fn forward_without_listener_is_silent_and_fast() {
    let dir = scratch("forward");
    let started = Instant::now();
    let mut child = Command::new(PROBE)
        .args(["hook-forward", "--agent", "claude", "--socket"])
        .arg(dir.join("absent.sock"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(br#"{"session_id":"s","hook_event_name":"Stop"}"#)
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let elapsed = started.elapsed();
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    // Includes starting the process; macOS adds about 0.6 s to a binary's first run
    // after it is built.
    assert!(elapsed < Duration::from_secs(1), "took {elapsed:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn status_line_prints_the_owner_output_and_records_limits() {
    let data_home = short_data_home("statusline");
    let mut child = Command::new(PROBE)
        .args(["hook-statusline", "--chain", "printf 'owner:'; cat"])
        .env("XDG_DATA_HOME", &data_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let input = r#"{"session_id":"s","rate_limits":{"five_hour":{"used_percentage":12,"resets_at":4000000000}}}"#;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        format!("owner:{input}")
    );

    let snapshot = data_home
        .join("headquarter")
        .join("quota")
        .join("claude.json");
    let saved: Value = serde_json::from_slice(&std::fs::read(snapshot).unwrap()).unwrap();
    assert_eq!(saved["windows"][0]["used_percent"], 12.0);
    assert!(!saved.to_string().contains("session_id"));
    let _ = std::fs::remove_dir_all(&data_home);
}
