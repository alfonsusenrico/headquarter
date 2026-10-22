//! Codex sub-agents still running for a session. A sub-agent writes its own rollout
//! whose `session_meta` names the parent thread
//! (`payload.source.subagent.thread_spawn.parent_thread_id`); it is running while its
//! latest turn event is `task_started`. Only event types and ids are read, never message
//! text.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::external::{collect_files, rollout_tail};

/// A sub-agent whose rollout has not changed for this long is not counted as running:
/// its server may have stopped without recording the end of its turn.
const STALE_AFTER: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunningSubagent {
    pub thread_id: String,
    pub turn_id: String,
}

/// Running sub-agents of `parents` (Codex thread ids), by parent. Rollouts are looked up
/// under `sessions_dir` among files changed within the last 15 minutes.
pub fn running_subagents(
    sessions_dir: &Path,
    parents: &[String],
) -> HashMap<String, Vec<RunningSubagent>> {
    let mut found: HashMap<String, Vec<RunningSubagent>> = HashMap::new();
    if parents.is_empty() {
        return found;
    }
    let cutoff = SystemTime::now()
        .checked_sub(STALE_AFTER)
        .unwrap_or(SystemTime::UNIX_EPOCH);
    for path in recent_rollouts(sessions_dir, cutoff) {
        let Some((thread_id, parent)) = subagent_meta(&path) else {
            continue;
        };
        if !parents.contains(&parent) {
            continue;
        }
        let Ok(tail) = rollout_tail(&path) else {
            continue;
        };
        if let Some(turn_id) = running_turn(&tail) {
            found
                .entry(parent)
                .or_default()
                .push(RunningSubagent { thread_id, turn_id });
        }
    }
    found
}

fn recent_rollouts(sessions_dir: &Path, cutoff: SystemTime) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files(sessions_dir, &mut files, &|path| {
        path.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("rollout-"))
            && path.extension().is_some_and(|ext| ext == "jsonl")
    });
    files.retain(|path| {
        std::fs::metadata(path)
            .and_then(|meta| meta.modified())
            .is_ok_and(|modified| modified >= cutoff)
    });
    files
}

/// The sub-agent's thread id and its parent's, from the first line.
fn subagent_meta(path: &Path) -> Option<(String, String)> {
    let mut first = String::new();
    BufReader::new(File::open(path).ok()?)
        .read_line(&mut first)
        .ok()?;
    parse_meta(&first)
}

fn parse_meta(line: &str) -> Option<(String, String)> {
    let value: Value = serde_json::from_str(line).ok()?;
    if value.get("type").and_then(Value::as_str) != Some("session_meta") {
        return None;
    }
    let payload = value.get("payload")?;
    let parent = payload
        .pointer("/source/subagent/thread_spawn/parent_thread_id")?
        .as_str()?;
    let thread = payload.get("id")?.as_str()?;
    Some((thread.to_owned(), parent.to_owned()))
}

/// The id of the latest turn when it has started and not ended.
fn running_turn(tail: &str) -> Option<String> {
    tail.lines().rev().find_map(|line| {
        let value: Value = serde_json::from_str(line).ok()?;
        if value.get("type").and_then(Value::as_str) != Some("event_msg") {
            return None;
        }
        let payload = value.get("payload")?;
        match payload.get("type")?.as_str()? {
            "task_started" => Some(
                payload
                    .get("turn_id")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
            ),
            "task_complete" | "turn_aborted" => Some(None),
            _ => None,
        }
    })?
}

#[cfg(test)]
mod tests {
    use super::*;

    const PARENT: &str = "01a15bf0-d0b1-78f0-a1c2-1219b7c10a89";

    fn meta(id: &str, parent: Option<&str>) -> String {
        let source = match parent {
            Some(parent) => format!(
                r#"{{"subagent":{{"thread_spawn":{{"parent_thread_id":"{parent}","depth":1}}}}}}"#
            ),
            None => r#""vscode""#.to_owned(),
        };
        format!(
            r#"{{"type":"session_meta","payload":{{"id":"{id}","cwd":"/w","source":{source}}}}}"#
        )
    }

    fn event(kind: &str, turn: &str) -> String {
        format!(r#"{{"type":"event_msg","payload":{{"type":"{kind}","turn_id":"{turn}"}}}}"#)
    }

    #[test]
    fn running_finished_and_unrelated_rollouts() {
        let dir = std::env::temp_dir().join(format!("hq-subagents-{}", std::process::id()));
        let day = dir.join("2026/10/21");
        std::fs::create_dir_all(&day).unwrap();
        let write = |name: &str, lines: &[String]| {
            std::fs::write(day.join(format!("rollout-{name}.jsonl")), lines.join("\n")).unwrap();
        };
        let message = r#"{"type":"response_item","payload":{"type":"message","content":"SECRET"}}"#;
        write("parent", &[meta(PARENT, None), event("task_started", "p1")]);
        write(
            "running",
            &[
                meta("sub-1", Some(PARENT)),
                event("task_started", "t1"),
                message.to_owned(),
            ],
        );
        write(
            "finished",
            &[
                meta("sub-2", Some(PARENT)),
                event("task_started", "t2"),
                event("task_complete", "t2"),
            ],
        );
        write(
            "aborted",
            &[
                meta("sub-3", Some(PARENT)),
                event("task_started", "t3"),
                event("turn_aborted", "t3"),
            ],
        );
        write(
            "other",
            &[meta("sub-4", Some("another")), event("task_started", "t4")],
        );

        let found = running_subagents(&dir, &[PARENT.to_owned()]);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(
            found.get(PARENT),
            Some(&vec![RunningSubagent {
                thread_id: "sub-1".into(),
                turn_id: "t1".into()
            }])
        );
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn no_parents_reads_nothing() {
        assert!(running_subagents(Path::new("/nonexistent"), &[]).is_empty());
    }

    #[test]
    fn meta_of_a_top_level_thread_is_not_a_subagent() {
        assert_eq!(parse_meta(&meta(PARENT, None)), None);
        assert_eq!(
            parse_meta(&meta("sub", Some(PARENT))),
            Some(("sub".to_owned(), PARENT.to_owned()))
        );
    }
}
