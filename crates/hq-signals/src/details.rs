//! Session details for the terminal pane's header
//! (`openspec/changes/add-session-header/design.md`, D2): numbers and names each agent
//! reports about its session. Only these fields are read; prompt and output text never
//! are, and the details are kept in memory only.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Model names longer than this are not names; they are dropped.
const MAX_MODEL_CHARS: usize = 80;
/// A rollout poll reads at most this much new text, so a huge burst cannot stall it.
const MAX_READ: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionDetails {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_used_percent: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_left_percent: Option<f64>,
    /// The model's context window, in tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens: Option<TokenCounts>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// Token counts as the agent reports them; any may be missing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenCounts {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_write: Option<u64>,
}

impl TokenCounts {
    fn is_empty(&self) -> bool {
        *self == TokenCounts::default()
    }
}

impl SessionDetails {
    /// The details with every value checked again: they cross a trust boundary (the
    /// hooks socket) between the helper and the host.
    pub fn sanitized(self) -> SessionDetails {
        SessionDetails {
            model: self.model.as_deref().and_then(model_name),
            context_used_percent: self.context_used_percent.and_then(percent),
            context_left_percent: self.context_left_percent.and_then(percent),
            context_window: self.context_window.filter(|window| *window > 0),
            tokens: self.tokens.filter(|tokens| !tokens.is_empty()),
            cost_usd: self
                .cost_usd
                .filter(|cost| cost.is_finite() && *cost >= 0.0),
            branch: self.branch.as_deref().and_then(branch_name),
        }
    }
}

fn percent(value: f64) -> Option<f64> {
    (value.is_finite() && (0.0..=100.0).contains(&value)).then_some(value)
}

fn model_name(text: &str) -> Option<String> {
    let text = text.trim();
    let fits = !text.is_empty()
        && text.chars().count() <= MAX_MODEL_CHARS
        && !text.chars().any(char::is_control);
    fits.then(|| text.to_owned())
}

fn branch_name(text: &str) -> Option<String> {
    let text = text.trim();
    let fits = !text.is_empty() && text.len() <= 255 && !text.chars().any(char::is_control);
    fits.then(|| text.to_owned())
}

fn number(value: &Value, path: &[&str]) -> Option<f64> {
    path.iter()
        .try_fold(value, |value, key| value.get(key))?
        .as_f64()
}

fn count(value: &Value, path: &[&str]) -> Option<u64> {
    path.iter()
        .try_fold(value, |value, key| value.get(key))?
        .as_u64()
}

fn text<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    path.iter()
        .try_fold(value, |value, key| value.get(key))?
        .as_str()
}

/// The details in Claude Code's status-line input. Fields Claude leaves `null` early in
/// a session, or after `/compact`, are simply missing.
pub fn from_claude_status_line(value: &Value) -> SessionDetails {
    let usage = |key| count(value, &["context_window", "current_usage", key]);
    let tokens = TokenCounts {
        input: count(value, &["context_window", "total_input_tokens"]),
        output: count(value, &["context_window", "total_output_tokens"]),
        cache_read: usage("cache_read_input_tokens"),
        cache_write: usage("cache_creation_input_tokens"),
    };
    SessionDetails {
        model: text(value, &["model", "display_name"])
            .or_else(|| text(value, &["model", "id"]))
            .map(str::to_owned),
        context_used_percent: number(value, &["context_window", "used_percentage"]),
        context_left_percent: number(value, &["context_window", "remaining_percentage"]),
        context_window: count(value, &["context_window", "context_window_size"]),
        tokens: Some(tokens),
        cost_usd: number(value, &["cost", "total_cost_usd"]),
        branch: text(value, &["worktree", "branch"]).map(str::to_owned),
    }
    .sanitized()
}

/// Follows one Codex rollout file, keeping the latest model and token counts. Only
/// `turn_context` and `token_count` records are looked at; everything else, prompts and
/// replies included, is skipped unread.
pub struct CodexRollout {
    path: PathBuf,
    offset: u64,
    partial: Vec<u8>,
    details: SessionDetails,
}

impl CodexRollout {
    pub fn new(path: PathBuf) -> CodexRollout {
        CodexRollout {
            path,
            offset: 0,
            partial: Vec::new(),
            details: SessionDetails::default(),
        }
    }

    pub fn details(&self) -> &SessionDetails {
        &self.details
    }

    /// Reads what the file gained since the last poll; true when the details changed.
    pub fn poll(&mut self) -> bool {
        let Ok(mut file) = std::fs::File::open(&self.path) else {
            return false;
        };
        let length = file.metadata().map(|meta| meta.len()).unwrap_or(0);
        if length < self.offset {
            // Replaced or truncated: read it again from the start.
            self.offset = 0;
            self.partial.clear();
        }
        if file.seek(SeekFrom::Start(self.offset)).is_err() {
            return false;
        }
        let mut added = Vec::new();
        let Ok(read) = file.take(MAX_READ).read_to_end(&mut added) else {
            return false;
        };
        self.offset += read as u64;
        self.partial.extend_from_slice(&added);
        let Some(end) = self.partial.iter().rposition(|byte| *byte == b'\n') else {
            return false;
        };
        let complete: Vec<u8> = self.partial.drain(..=end).collect();
        let before = self.details.clone();
        for line in complete.split(|byte| *byte == b'\n') {
            apply_codex_line(line, &mut self.details);
        }
        self.details != before
    }
}

fn apply_codex_line(line: &[u8], details: &mut SessionDetails) {
    // Cheap test before parsing: most lines are conversation, never parsed here.
    let wanted = |needle: &[u8]| line.windows(needle.len()).any(|window| window == needle);
    if !wanted(b"\"turn_context\"") && !wanted(b"\"token_count\"") {
        return;
    }
    let Ok(value) = serde_json::from_slice::<Value>(line) else {
        return;
    };
    match value.get("type").and_then(Value::as_str) {
        Some("turn_context") => {
            if let Some(model) = text(&value, &["payload", "model"]).and_then(model_name) {
                details.model = Some(model);
            }
        }
        Some("event_msg") if text(&value, &["payload", "type"]) == Some("token_count") => {
            let info = &value["payload"]["info"];
            let window = count(info, &["model_context_window"]).filter(|window| *window > 0);
            let total = |key| count(info, &["total_token_usage", key]);
            let last = |key| count(info, &["last_token_usage", key]);
            // Tokens in the window after the last turn: everything but reasoning, which
            // Codex drops between turns (Codex's own `tokens_in_context_window`).
            let in_window = last("total_tokens")
                .map(|all| all.saturating_sub(last("reasoning_output_tokens").unwrap_or(0)));
            if let (Some(window), Some(used)) = (window, in_window) {
                let share = (used as f64 / window as f64 * 100.0).clamp(0.0, 100.0);
                details.context_used_percent = Some(share);
                details.context_left_percent = Some(100.0 - share);
                details.context_window = Some(window);
            }
            let tokens = TokenCounts {
                input: total("input_tokens"),
                output: total("output_tokens"),
                cache_read: total("cached_input_tokens"),
                cache_write: total("cache_write_input_tokens"),
            };
            if !tokens.is_empty() {
                details.tokens = Some(tokens);
            }
        }
        _ => {}
    }
}

/// The Codex rollout file of a conversation: `rollout-<time>-<id>.jsonl` under
/// `<codex home>/sessions`.
pub fn codex_rollout(codex_home: &Path, conversation: &str) -> Option<PathBuf> {
    fn search(dir: &Path, suffix: &str) -> Option<PathBuf> {
        let mut entries: Vec<_> = std::fs::read_dir(dir).ok()?.flatten().collect();
        // Newest folders first: a running session's file is in today's folder.
        entries.sort_by_key(|entry| std::cmp::Reverse(entry.file_name()));
        for entry in entries {
            let path = entry.path();
            match entry.file_type() {
                Ok(kind) if kind.is_dir() => {
                    if let Some(found) = search(&path, suffix) {
                        return Some(found);
                    }
                }
                Ok(_) => {
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    if name.starts_with("rollout-") && name.ends_with(suffix) {
                        return Some(path);
                    }
                }
                Err(_) => {}
            }
        }
        None
    }
    search(
        &codex_home.join("sessions"),
        &format!("-{conversation}.jsonl"),
    )
}

/// The current git branch of `folder` or the nearest folder above it with a `.git`,
/// read from `HEAD` without starting git: a branch name, the first 7 characters of a
/// detached commit, or `None` when there is no readable repository.
pub fn git_branch(folder: &Path) -> Option<String> {
    let mut dir = Some(folder);
    while let Some(here) = dir {
        let dot_git = here.join(".git");
        if dot_git.is_dir() {
            return head_branch(&dot_git);
        }
        if dot_git.is_file() {
            // A worktree or submodule: `.git` names the real git folder.
            let pointer = std::fs::read_to_string(&dot_git).ok()?;
            let target = pointer.trim().strip_prefix("gitdir:")?.trim();
            let target = here.join(target);
            return head_branch(&target);
        }
        dir = here.parent();
    }
    None
}

fn head_branch(git_dir: &Path) -> Option<String> {
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref: ") {
        Some(reference) => {
            let name = reference.strip_prefix("refs/heads/").unwrap_or(reference);
            branch_name(name)
        }
        None if head.len() >= 7 && head.chars().all(|c| c.is_ascii_hexdigit()) => {
            Some(head[..7].to_owned())
        }
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn folder(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hq-details-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn claude_status_line_fields() {
        let value = serde_json::json!({
            "model": {"id": "claude-x", "display_name": "Opus"},
            "cost": {"total_cost_usd": 0.42},
            "context_window": {
                "total_input_tokens": 15234, "total_output_tokens": 4521,
                "context_window_size": 200000,
                "used_percentage": 62.0, "remaining_percentage": 38.0,
                "current_usage": {"input_tokens": 8500, "output_tokens": 1200,
                    "cache_creation_input_tokens": 5000, "cache_read_input_tokens": 2000}
            },
            "transcript_path": "/never/read.jsonl"
        });
        let details = from_claude_status_line(&value);
        assert_eq!(details.model.as_deref(), Some("Opus"));
        assert_eq!(details.context_used_percent, Some(62.0));
        assert_eq!(details.context_left_percent, Some(38.0));
        assert_eq!(details.context_window, Some(200_000));
        assert_eq!(details.cost_usd, Some(0.42));
        let tokens = details.tokens.unwrap();
        assert_eq!((tokens.input, tokens.output), (Some(15234), Some(4521)));
        assert_eq!(
            (tokens.cache_read, tokens.cache_write),
            (Some(2000), Some(5000))
        );
    }

    #[test]
    fn claude_early_in_a_session() {
        let value = serde_json::json!({
            "model": {"display_name": "Sonnet"},
            "context_window": {"used_percentage": null, "current_usage": null}
        });
        let details = from_claude_status_line(&value);
        assert_eq!(details.model.as_deref(), Some("Sonnet"));
        assert_eq!(details.context_used_percent, None);
        assert_eq!(details.tokens, None);
        assert_eq!(details.cost_usd, None);
    }

    #[test]
    fn out_of_range_values_are_dropped() {
        let details = SessionDetails {
            model: Some("x\u{7}".into()),
            context_used_percent: Some(140.0),
            context_left_percent: Some(f64::NAN),
            context_window: Some(0),
            tokens: Some(TokenCounts::default()),
            cost_usd: Some(-1.0),
            branch: Some(" ".into()),
        }
        .sanitized();
        assert_eq!(details, SessionDetails::default());
    }

    #[test]
    fn codex_rollout_is_followed_by_record_type() {
        let dir = folder("codex");
        let path = dir.join("rollout-2026-10-20T10-00-00-abc.jsonl");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            r#"{{"type":"session_meta","payload":{{"id":"abc"}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"response_item","payload":{{"content":"PROMPT-SECRET"}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"turn_context","payload":{{"model":"gpt-5-codex","cwd":"/w"}}}}"#
        )
        .unwrap();
        let mut rollout = CodexRollout::new(path.clone());
        assert!(rollout.poll());
        assert_eq!(rollout.details().model.as_deref(), Some("gpt-5-codex"));
        assert!(!rollout.poll(), "nothing new");

        // A record arriving in two writes is read once it is complete.
        write!(
            file,
            r#"{{"type":"event_msg","payload":{{"type":"token_count","info":{{"model_context_window":200000,"#
        )
        .unwrap();
        file.flush().unwrap();
        assert!(!rollout.poll());
        writeln!(
            file,
            r#""last_token_usage":{{"total_tokens":60000,"reasoning_output_tokens":10000}},"total_token_usage":{{"input_tokens":90000,"output_tokens":7000,"cached_input_tokens":40000}}}}}}}}"#
        )
        .unwrap();
        assert!(rollout.poll());
        let details = rollout.details();
        assert_eq!(details.context_used_percent, Some(25.0));
        assert_eq!(details.context_left_percent, Some(75.0));
        assert_eq!(details.context_window, Some(200_000));
        let tokens = details.tokens.clone().unwrap();
        assert_eq!(
            (tokens.input, tokens.cache_read),
            (Some(90_000), Some(40_000))
        );
        assert!(format!("{details:?}").find("PROMPT-SECRET").is_none());

        assert_eq!(
            codex_rollout(&dir.join(".."), "abc"),
            None,
            "only under sessions/"
        );
        let home = folder("codex-home");
        let day = home.join("sessions/2026/10/20");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::copy(&path, day.join("rollout-2026-10-20T10-00-00-abc.jsonl")).unwrap();
        assert!(codex_rollout(&home, "abc").is_some());
        assert_eq!(codex_rollout(&home, "zzz"), None);
    }

    #[test]
    fn git_branches_from_head() {
        let repo = folder("git");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::write(repo.join(".git/HEAD"), "ref: refs/heads/feat/x\n").unwrap();
        let inner = repo.join("src/deep");
        std::fs::create_dir_all(&inner).unwrap();
        assert_eq!(git_branch(&inner).as_deref(), Some("feat/x"));

        std::fs::write(repo.join(".git/HEAD"), "0123456789abcdef0123\n").unwrap();
        assert_eq!(git_branch(&repo).as_deref(), Some("0123456"));

        let worktree = folder("git-worktree");
        let real = repo.join(".git/worktrees/w");
        std::fs::create_dir_all(&real).unwrap();
        std::fs::write(real.join("HEAD"), "ref: refs/heads/wt\n").unwrap();
        std::fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", real.display()),
        )
        .unwrap();
        assert_eq!(git_branch(&worktree).as_deref(), Some("wt"));

        let plain = folder("no-git");
        assert_eq!(git_branch(&plain), None);
    }
}
