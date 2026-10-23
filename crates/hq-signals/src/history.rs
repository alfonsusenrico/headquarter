//! Read-only readers for the agents' own conversation history.
//!
//! Each reader yields [`Conversation`]s: id, folder, times, and the agent's title.
//! Titles are returned for display only; callers must not persist them (owner
//! decision 2026-10-20). Agent files are never written, and fields that hold prompt
//! text (agy's `preview`, Claude's messages) are never read into a result.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::event::Agent;
use crate::paths;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Conversation {
    pub agent: Agent,
    /// The agent's own conversation id (what its resume takes).
    pub id: String,
    pub cwd: String,
    /// Unix milliseconds.
    pub started_at: u64,
    pub last_activity_at: u64,
    /// For display only; never stored.
    pub title: Option<String>,
}

#[derive(Debug, Default)]
pub struct ScanReport {
    pub conversations: Vec<Conversation>,
    /// Files or rows that could not be read this pass.
    pub skipped: usize,
    /// Sources actually parsed this pass (unchanged ones are served from cache).
    pub parsed: usize,
}

/// Where each agent keeps its history.
#[derive(Debug, Clone, Default)]
pub struct HistoryRoots {
    pub claude_projects: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub agy_dir: Option<PathBuf>,
}

impl HistoryRoots {
    pub fn standard() -> HistoryRoots {
        HistoryRoots {
            claude_projects: paths::claude_config_dir().map(|dir| dir.join("projects")),
            codex_home: paths::codex_home(),
            agy_dir: paths::agy_dir(),
        }
    }
}

type Stamp = (SystemTime, u64);

/// Keeps what earlier passes read so unchanged sources are not parsed again.
pub struct HistoryReader {
    roots: HistoryRoots,
    files: HashMap<PathBuf, (Stamp, Option<Conversation>)>,
    codex_titles: (Option<Stamp>, HashMap<String, String>),
    agy: (Option<Stamp>, Vec<Conversation>),
}

impl HistoryReader {
    pub fn new(roots: HistoryRoots) -> HistoryReader {
        HistoryReader {
            roots,
            files: HashMap::new(),
            codex_titles: (None, HashMap::new()),
            agy: (None, Vec::new()),
        }
    }

    pub fn scan(&mut self) -> ScanReport {
        let mut report = ScanReport::default();
        if let Some(projects) = self.roots.claude_projects.clone() {
            for file in claude_files(&projects) {
                self.read_file(&file, Agent::Claude, &mut report);
            }
        }
        if let Some(home) = self.roots.codex_home.clone() {
            self.refresh_codex_titles(&home, &mut report);
            let mut files = Vec::new();
            collect_rollouts(&home.join("sessions"), &mut files);
            for file in files {
                self.read_file(&file, Agent::Codex, &mut report);
            }
            for conversation in report
                .conversations
                .iter_mut()
                .filter(|conversation| conversation.agent == Agent::Codex)
            {
                conversation.title = self.codex_titles.1.get(&conversation.id).cloned();
            }
        }
        if let Some(dir) = self.roots.agy_dir.clone() {
            self.refresh_agy(&dir, &mut report);
            report.conversations.extend(self.agy.1.iter().cloned());
        }
        report
    }

    fn read_file(&mut self, path: &Path, agent: Agent, report: &mut ScanReport) {
        let Some(current) = stamp(path) else {
            return;
        };
        let cached = self
            .files
            .get(path)
            .filter(|(seen, _)| *seen == current)
            .map(|(_, conversation)| conversation.clone());
        let conversation = match cached {
            Some(conversation) => conversation,
            None => {
                report.parsed += 1;
                let read = match agent {
                    Agent::Claude => read_claude(path),
                    Agent::Codex => read_codex_rollout(path),
                    Agent::Agy => None,
                };
                self.files.insert(path.to_owned(), (current, read.clone()));
                read
            }
        };
        match conversation {
            Some(conversation) => report.conversations.push(conversation),
            None => report.skipped += 1,
        }
    }

    fn refresh_codex_titles(&mut self, home: &Path, report: &mut ScanReport) {
        let index = home.join("session_index.jsonl");
        let current = stamp(&index);
        if current.is_none() || current == self.codex_titles.0 {
            return;
        }
        report.parsed += 1;
        let mut titles = HashMap::new();
        if let Ok(file) = File::open(&index) {
            for line in BufReader::new(file).lines().map_while(Result::ok) {
                let Ok(entry) = serde_json::from_str::<Value>(&line) else {
                    report.skipped += 1;
                    continue;
                };
                if let (Some(id), Some(title)) = (text(&entry, "id"), text(&entry, "thread_name")) {
                    // A later line for the same thread is a rename.
                    titles.insert(id, title);
                }
            }
        }
        self.codex_titles = (current, titles);
    }

    fn refresh_agy(&mut self, dir: &Path, report: &mut ScanReport) {
        let database = dir.join("conversation_summaries.db");
        // Writes may land in the WAL file first, so its change counts too.
        let current = newest_stamp(&[database.clone(), database.with_extension("db-wal")]);
        if current.is_none() || current == self.agy.0 {
            return;
        }
        report.parsed += 1;
        match read_agy(&database, &dir.join("conversations")) {
            Ok((conversations, skipped)) => {
                report.skipped += skipped;
                self.agy = (current, conversations);
            }
            Err(_) => {
                // Locked or unreadable now: keep the last good read, retry next pass.
                report.skipped += 1;
            }
        }
    }
}

fn stamp(path: &Path) -> Option<Stamp> {
    let metadata = std::fs::metadata(path).ok()?;
    Some((metadata.modified().ok()?, metadata.len()))
}

fn newest_stamp(paths: &[PathBuf]) -> Option<Stamp> {
    paths.iter().filter_map(|path| stamp(path)).max()
}

fn millis(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}

fn born_and_modified(path: &Path) -> Option<(u64, u64)> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = millis(metadata.modified().ok()?);
    let born = metadata.created().map(millis).unwrap_or(modified);
    Some((born.min(modified), modified))
}

/// When the conversation last had a real message: the timestamp of the last line in
/// the file's tail that `is_message` accepts. Agents also write bookkeeping lines when
/// they start and stop (Claude's `cost-state` and `mode` have no timestamp), which move
/// the file's modification time without any activity. Only the line's `"timestamp"` is
/// read; message text is never parsed.
fn last_message_time(path: &Path, is_message: fn(&str) -> bool) -> Option<u64> {
    let tail = crate::external::rollout_tail(path).ok()?;
    tail.lines()
        .rev()
        .filter(|line| is_message(line))
        .find_map(|line| {
            let start = line.find("\"timestamp\":\"")? + "\"timestamp\":\"".len();
            let end = start + line[start..].find('"')?;
            parse_utc_timestamp(&line[start..end])
        })
}

fn claude_message(line: &str) -> bool {
    line.contains("\"type\":\"user\"") || line.contains("\"type\":\"assistant\"")
}

fn codex_message(line: &str) -> bool {
    line.contains("\"type\":\"response_item\"")
        || (line.contains("\"type\":\"event_msg\"")
            && [
                "task_started",
                "task_complete",
                "user_message",
                "agent_message",
            ]
            .iter()
            .any(|kind| line.contains(kind)))
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

/// Top-level session files only: subagent transcripts live in nested folders.
fn claude_files(projects: &Path) -> Vec<PathBuf> {
    let Ok(folders) = std::fs::read_dir(projects) else {
        return Vec::new();
    };
    folders
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| std::fs::read_dir(entry.path()).ok())
        .flat_map(|files| files.flatten())
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "jsonl"))
        .collect()
}

fn read_claude(path: &Path) -> Option<Conversation> {
    let id = path.file_stem()?.to_str()?.to_owned();
    let (started_at, modified) = born_and_modified(path)?;
    let last_activity_at =
        last_message_time(path, claude_message).map_or(modified, |at| at.min(modified));
    let mut cwd = None;
    let mut ai_title = None;
    let mut agent_name = None;
    for line in BufReader::new(File::open(path).ok()?)
        .lines()
        .map_while(Result::ok)
    {
        // Check cheaply before parsing; most lines are messages we never need.
        let wants_cwd = cwd.is_none() && line.contains("\"cwd\"");
        let wants_title = line.contains("ai-title") || line.contains("agent-name");
        if !wants_cwd && !wants_title {
            continue;
        }
        let Ok(record) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if wants_cwd {
            cwd = text(&record, "cwd");
        }
        match record.get("type").and_then(Value::as_str) {
            Some("ai-title") => ai_title = text(&record, "aiTitle").or(ai_title),
            Some("agent-name") => agent_name = text(&record, "agentName").or(agent_name),
            _ => {}
        }
    }
    Some(Conversation {
        agent: Agent::Claude,
        id,
        cwd: cwd?,
        started_at,
        last_activity_at,
        title: agent_name.or(ai_title),
    })
}

fn collect_rollouts(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => collect_rollouts(&path, out),
            Ok(kind)
                if kind.is_file()
                    && path
                        .file_name()
                        .is_some_and(|name| name.to_string_lossy().starts_with("rollout-"))
                    && path.extension().is_some_and(|ext| ext == "jsonl") =>
            {
                out.push(path);
            }
            _ => {}
        }
    }
}

fn read_codex_rollout(path: &Path) -> Option<Conversation> {
    let (started_at, modified) = born_and_modified(path)?;
    let last_activity_at =
        last_message_time(path, codex_message).map_or(modified, |at| at.min(modified));
    let mut first = String::new();
    BufReader::new(File::open(path).ok()?)
        .read_line(&mut first)
        .ok()?;
    let line: Value = serde_json::from_str(&first).ok()?;
    if line.get("type").and_then(Value::as_str) != Some("session_meta") {
        return None;
    }
    let payload = line.get("payload")?;
    Some(Conversation {
        agent: Agent::Codex,
        id: text(payload, "session_id").or_else(|| text(payload, "id"))?,
        cwd: text(payload, "cwd")?,
        started_at,
        last_activity_at,
        title: None,
    })
}

/// Reads top-level agy conversations. `preview` (prompt text) is never selected.
fn read_agy(
    database: &Path,
    conversations_dir: &Path,
) -> rusqlite::Result<(Vec<Conversation>, usize)> {
    let connection = rusqlite::Connection::open_with_flags(
        database,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(std::time::Duration::from_millis(500))?;
    let mut statement = connection.prepare(
        "SELECT conversation_id, title, workspace_uris, last_modified_time
         FROM conversation_summaries
         WHERE COALESCE(nesting_depth, 0) = 0",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, Option<String>>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut conversations = Vec::new();
    let mut skipped = 0;
    for row in rows {
        let Ok((Some(id), title, workspaces, modified)) = row else {
            skipped += 1;
            continue;
        };
        let Some(cwd) = workspaces.as_deref().and_then(first_file_uri) else {
            skipped += 1;
            continue;
        };
        let Some(last_activity_at) = modified.as_deref().and_then(parse_utc_timestamp) else {
            skipped += 1;
            continue;
        };
        let started_at = conversations_dir
            .join(format!("{id}.db"))
            .metadata()
            .ok()
            .and_then(|metadata| metadata.created().ok())
            .map_or(last_activity_at, millis)
            .min(last_activity_at);
        conversations.push(Conversation {
            agent: Agent::Agy,
            id,
            cwd,
            started_at,
            last_activity_at,
            title: title.filter(|title| !title.trim().is_empty()),
        });
    }
    Ok((conversations, skipped))
}

/// The first `file://` path in a JSON array of URIs, percent-decoded.
fn first_file_uri(workspaces: &str) -> Option<String> {
    let uris: Vec<String> = serde_json::from_str(workspaces).ok()?;
    uris.iter()
        .find_map(|uri| uri.strip_prefix("file://"))
        .and_then(percent_decode)
        .filter(|path| path.starts_with('/'))
}

fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Parses `YYYY-MM-DD HH:MM:SS[.ffffff]+00:00` (UTC, as agy writes it) to Unix ms.
fn parse_utc_timestamp(text: &str) -> Option<u64> {
    let text = text.trim();
    let (date, rest) = text.split_once([' ', 'T'])?;
    let time = rest
        .strip_suffix("+00:00")
        .or_else(|| rest.strip_suffix('Z'))?;
    let mut date_parts = date.split('-').map(str::parse::<i64>);
    let (year, month, day) = (
        date_parts.next()?.ok()?,
        date_parts.next()?.ok()?,
        date_parts.next()?.ok()?,
    );
    let (clock, fraction) = time.split_once('.').unwrap_or((time, "0"));
    let mut clock_parts = clock.split(':').map(str::parse::<i64>);
    let (hour, minute, second) = (
        clock_parts.next()?.ok()?,
        clock_parts.next()?.ok()?,
        clock_parts.next()?.ok()?,
    );
    let millis_part: i64 = format!("{fraction:0<3}")[..3].parse().ok()?;
    let days = days_from_civil(year, month, day);
    let total = ((days * 24 + hour) * 60 + minute) * 60 + second;
    u64::try_from(total * 1000 + millis_part).ok()
}

/// Days since 1970-01-01 for a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        root: PathBuf,
    }

    impl Fixture {
        fn new(name: &str) -> Fixture {
            let root =
                std::env::temp_dir().join(format!("hq-history-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            Fixture { root }
        }

        fn write(&self, relative: &str, content: &str) -> PathBuf {
            let path = self.root.join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, content).unwrap();
            path
        }

        fn reader(&self) -> HistoryReader {
            HistoryReader::new(HistoryRoots {
                claude_projects: Some(self.root.join("claude/projects")),
                codex_home: Some(self.root.join("codex")),
                agy_dir: Some(self.root.join("agy")),
            })
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    const SECRET: &str = "PROMPT-SECRET-77";

    #[test]
    fn claude_reads_folder_and_prefers_the_agent_name() {
        let fixture = Fixture::new("claude");
        fixture.write(
            "claude/projects/-w-app/s1.jsonl",
            &[
                r#"{"type":"mode","mode":"normal"}"#.to_owned(),
                format!(r#"{{"type":"user","cwd":"/w/app","message":{{"content":"{SECRET}"}}}}"#),
                "not json at all".to_owned(),
                r#"{"type":"ai-title","aiTitle":"Fix login","sessionId":"s1"}"#.to_owned(),
                r#"{"type":"agent-name","agentName":"landing-hero","sessionId":"s1"}"#.to_owned(),
                r#"{"type":"ai-title","aiTitle":"Later title","sessionId":"s1"}"#.to_owned(),
            ]
            .join("\n"),
        );
        fixture.write(
            "claude/projects/-w-app/s1/subagents/agent-1.jsonl",
            r#"{"type":"user","cwd":"/w/app"}"#,
        );
        fixture.write("claude/projects/-w-app/notes.txt", "ignored");
        let report = fixture.reader().scan();
        assert_eq!(report.conversations.len(), 1, "{report:?}");
        let conversation = &report.conversations[0];
        assert_eq!(conversation.id, "s1");
        assert_eq!(conversation.cwd, "/w/app");
        assert_eq!(conversation.title.as_deref(), Some("landing-hero"));
        assert!(!format!("{conversation:?}").contains(SECRET));
    }

    /// Claude appends bookkeeping without timestamps when it starts and stops; the time
    /// of the last message wins over the file's modification time.
    #[test]
    fn last_activity_is_the_last_message_not_the_last_write() {
        let fixture = Fixture::new("claude-time");
        fixture.write(
            "claude/projects/-w-app/s3.jsonl",
            &[
                format!(
                    r#"{{"type":"user","cwd":"/w/app","timestamp":"2026-10-21T02:00:00.000Z","message":{{"content":"{SECRET}"}}}}"#
                ),
                r#"{"type":"assistant","timestamp":"2026-10-21T02:00:05.500Z","message":{}}"#
                    .to_owned(),
                r#"{"type":"cost-state","sessionId":"s3"}"#.to_owned(),
                r#"{"type":"mode","mode":"normal"}"#.to_owned(),
            ]
            .join("\n"),
        );
        fixture.write(
            "codex/sessions/2026/10/21/rollout-x.jsonl",
            &[
                r#"{"type":"session_meta","payload":{"id":"t1","cwd":"/w/app"}}"#,
                r#"{"timestamp":"2026-10-21T03:00:00.000Z","type":"event_msg","payload":{"type":"task_complete"}}"#,
                r#"{"timestamp":"2026-10-21T03:30:00.000Z","type":"turn_context","payload":{}}"#,
            ]
            .join("\n"),
        );
        let report = fixture.reader().scan();
        let at = |agent: Agent| {
            report
                .conversations
                .iter()
                .find(|conversation| conversation.agent == agent)
                .unwrap()
                .last_activity_at
        };
        assert_eq!(at(Agent::Claude), 1_792_548_005_500);
        assert_eq!(at(Agent::Codex), 1_792_551_600_000);
    }

    #[test]
    fn claude_without_a_folder_is_skipped() {
        let fixture = Fixture::new("claude-skip");
        fixture.write("claude/projects/x/s2.jsonl", r#"{"type":"mode"}"#);
        let report = fixture.reader().scan();
        assert!(report.conversations.is_empty());
        assert_eq!(report.skipped, 1);
    }

    #[test]
    fn codex_reads_rollouts_and_latest_titles() {
        let fixture = Fixture::new("codex");
        fixture.write(
            "codex/sessions/2026/10/20/rollout-a.jsonl",
            &[
                r#"{"type":"session_meta","payload":{"session_id":"t1","id":"t1","cwd":"/w/api","base_instructions":"x"}}"#.to_owned(),
                format!(r#"{{"type":"response_item","payload":{{"content":"{SECRET}"}}}}"#),
            ]
            .join("\n"),
        );
        fixture.write(
            "codex/sessions/2026/10/20/rollout-b.jsonl",
            r#"{"type":"event_msg","payload":{}}"#,
        );
        fixture.write(
            "codex/session_index.jsonl",
            &[
                r#"{"id":"t1","thread_name":"First name","updated_at":"x"}"#,
                "garbage",
                r#"{"id":"t1","thread_name":"Renamed","updated_at":"y"}"#,
            ]
            .join("\n"),
        );
        let report = fixture.reader().scan();
        assert_eq!(report.conversations.len(), 1, "{report:?}");
        let conversation = &report.conversations[0];
        assert_eq!(
            (conversation.id.as_str(), conversation.cwd.as_str()),
            ("t1", "/w/api")
        );
        assert_eq!(conversation.title.as_deref(), Some("Renamed"));
        assert_eq!(
            report.skipped, 2,
            "rollout without meta and a bad index line"
        );
    }

    fn agy_database(fixture: &Fixture) -> PathBuf {
        let path = fixture.root.join("agy/conversation_summaries.db");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let connection = rusqlite::Connection::open(&path).unwrap();
        connection
            .execute_batch(&format!(
                "CREATE TABLE conversation_summaries (conversation_id TEXT, title TEXT NOT NULL DEFAULT '',
                     preview TEXT NOT NULL DEFAULT '', workspace_uris TEXT, last_modified_time datetime,
                     nesting_depth INTEGER, not_fully_idle numeric);
                 INSERT INTO conversation_summaries VALUES
                     ('c1', 'Ship hero', '{SECRET}', '[\"file:///w/my%20app\"]', '2026-10-20 04:47:39.447906+00:00', 0, 1),
                     ('c2', '', '{SECRET}', '[\"file:///w/site\"]', '2026-10-19 00:00:00+00:00', NULL, 0),
                     ('child', 'Sub', '{SECRET}', '[\"file:///w/site\"]', '2026-10-19 00:00:00+00:00', 1, 0),
                     ('c3', 'Bad', '{SECRET}', '[\"https://example.com\"]', '2026-10-19 00:00:00+00:00', 0, 0);"
            ))
            .unwrap();
        path
    }

    #[test]
    fn agy_reads_top_level_rows_without_previews() {
        let fixture = Fixture::new("agy");
        agy_database(&fixture);
        let report = fixture.reader().scan();
        let mut ids: Vec<&str> = report
            .conversations
            .iter()
            .map(|conversation| conversation.id.as_str())
            .collect();
        ids.sort_unstable();
        assert_eq!(ids, ["c1", "c2"]);
        assert_eq!(report.skipped, 1, "the non-file workspace");
        let c1 = report
            .conversations
            .iter()
            .find(|conversation| conversation.id == "c1")
            .unwrap();
        assert_eq!(c1.cwd, "/w/my app");
        assert_eq!(c1.title.as_deref(), Some("Ship hero"));
        assert_eq!(c1.last_activity_at, 1_792_471_659_447);
        let c2 = report
            .conversations
            .iter()
            .find(|conversation| conversation.id == "c2")
            .unwrap();
        assert_eq!(c2.title, None);
        assert!(!format!("{report:?}").contains(SECRET));
    }

    #[test]
    fn unchanged_sources_are_not_parsed_again() {
        let fixture = Fixture::new("incremental");
        fixture.write(
            "claude/projects/-w/s1.jsonl",
            r#"{"type":"user","cwd":"/w"}"#,
        );
        fixture.write(
            "codex/sessions/rollout-a.jsonl",
            r#"{"type":"session_meta","payload":{"session_id":"t1","cwd":"/w"}}"#,
        );
        fixture.write(
            "codex/session_index.jsonl",
            r#"{"id":"t1","thread_name":"T"}"#,
        );
        agy_database(&fixture);
        let mut reader = fixture.reader();
        let first = reader.scan();
        assert_eq!(first.parsed, 4);
        let second = reader.scan();
        assert_eq!(second.parsed, 0);
        assert_eq!(second.conversations.len(), first.conversations.len());
        assert_eq!(
            second
                .conversations
                .iter()
                .find(|conversation| conversation.id == "t1")
                .and_then(|conversation| conversation.title.as_deref()),
            Some("T"),
            "cached titles still apply"
        );

        fixture.write(
            "claude/projects/-w/s1.jsonl",
            "{\"type\":\"user\",\"cwd\":\"/w\"}\n{\"type\":\"ai-title\",\"aiTitle\":\"New\"}",
        );
        let third = reader.scan();
        assert_eq!(third.parsed, 1);
    }

    #[test]
    fn missing_sources_yield_nothing() {
        let fixture = Fixture::new("empty");
        let report = fixture.reader().scan();
        assert!(report.conversations.is_empty());
        assert_eq!(report.skipped, 0);
    }

    #[test]
    fn timestamps_and_uris() {
        assert_eq!(parse_utc_timestamp("1970-01-01 00:00:00+00:00"), Some(0));
        assert_eq!(
            parse_utc_timestamp("2026-10-20 04:47:39.447906+00:00"),
            Some(1_792_471_659_447)
        );
        assert_eq!(
            parse_utc_timestamp("2026-10-20T04:47:39Z"),
            Some(1_792_471_659_000)
        );
        assert_eq!(parse_utc_timestamp("2026-10-20 04:47:39+07:00"), None);
        assert_eq!(parse_utc_timestamp("yesterday"), None);
        assert_eq!(
            first_file_uri(r#"["file:///a/b%20c","file:///x"]"#).as_deref(),
            Some("/a/b c")
        );
        assert_eq!(first_file_uri(r#"["https://x"]"#), None);
        assert_eq!(first_file_uri("not json"), None);
    }
}
