//! Provider usage windows from documented local sources.
//!
//! - Codex: `account/rateLimits/read` on a short-lived `codex app-server`.
//! - Claude Code: `rate_limits` in the status line input of a session Headquarter
//!   launched, saved as a snapshot.
//! - agy: quota is only exposed through its file-configured status line.
//!
//! What the limits belong to (a subscription and its plan, an API key, a cloud
//! provider) comes from `account/read` for Codex and `claude auth status --json` for
//! Claude; agy does not report it. Only the account's kind and plan are kept: the
//! answers also carry the owner's email and organization, which are dropped unread.
//!
//! No source reads, copies, or sends provider credentials.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::event::Agent;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaWindow {
    pub duration_mins: Option<u64>,
    pub used_percent: f64,
    /// Unix seconds.
    pub resets_at: Option<u64>,
    /// The model a per-model window counts ("Fable"); `None` for all models.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

impl QuotaWindow {
    /// A window is stale when it has reset since the reading was taken.
    pub fn is_stale(&self, read_at_secs: u64, now_secs: u64) -> bool {
        self.resets_at
            .is_some_and(|reset| reset <= now_secs && read_at_secs < reset)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuotaReading {
    pub provider: Agent,
    pub windows: Vec<QuotaWindow>,
    /// Unix seconds.
    pub read_at: u64,
}

/// How the owner is signed in to an agent's provider, which decides what usage limits
/// apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Account {
    /// A subscription, with its plan as the provider names it ("max", "plus").
    Subscription {
        plan: Option<String>,
    },
    ApiKey,
    /// A cloud provider's API ("bedrock", "vertex").
    Cloud {
        provider: String,
    },
}

/// The account from Codex's `account/read` result: `{"account": {"type": "chatgpt",
/// "planType": "plus", ...}}` or `{"type": "apiKey"}`; `None` when signed out.
pub fn parse_codex_account(result: &Value) -> Option<Account> {
    let account = result.get("account")?;
    match account.get("type")?.as_str()? {
        "chatgpt" => Some(Account::Subscription {
            plan: account
                .get("planType")
                .and_then(Value::as_str)
                .map(str::to_owned),
        }),
        "apiKey" => Some(Account::ApiKey),
        _ => None,
    }
}

/// The account from `claude auth status --json` (fields seen in claude 2.1.304:
/// `loggedIn`, `authMethod`, `apiProvider`, `subscriptionType`); `None` when signed
/// out or the method is unknown.
pub fn parse_claude_auth_status(status: &Value) -> Option<Account> {
    if status.get("loggedIn").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    if let Some(provider) = status.get("apiProvider").and_then(Value::as_str)
        && provider != "firstParty"
    {
        return Some(Account::Cloud {
            provider: provider.to_owned(),
        });
    }
    match status.get("authMethod")?.as_str()? {
        "claude.ai" => Some(Account::Subscription {
            plan: status
                .get("subscriptionType")
                .and_then(Value::as_str)
                .map(str::to_owned),
        }),
        "oauth_token" => Some(Account::Subscription { plan: None }),
        "api_key" => Some(Account::ApiKey),
        _ => None,
    }
}

/// Runs `executable auth status --json`; `None` when it fails, is slow, or reports
/// no known account.
pub fn read_claude_account(executable: &Path, timeout: Duration) -> Option<Account> {
    let mut command = Command::new(executable);
    command
        .args(["auth", "status", "--json"])
        .env("PATH", hq_discovery::shell::agent_path(executable))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for marker in crate::adapters::PARENT_SESSION_MARKERS {
        command.env_remove(marker);
    }
    let mut child = command.spawn().ok()?;
    let stdout = child.stdout.take()?;
    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        let _ = std::io::Read::read_to_string(&mut BufReader::new(stdout), &mut text);
        let _ = done_tx.send(text);
    });
    let text = done_rx.recv_timeout(timeout).ok();
    let _ = child.kill();
    let _ = child.wait();
    parse_claude_auth_status(&serde_json::from_str(&text?).ok()?)
}

#[derive(Debug, Clone, PartialEq)]
pub enum QuotaReport {
    Available(QuotaReading),
    Unavailable { provider: Agent, reason: String },
}

const FIVE_HOURS_MINS: u64 = 5 * 60;
const WEEK_MINS: u64 = 7 * 24 * 60;

/// Human label for a window length: `5h`, `week`, `2d`, or minutes.
pub fn window_label(duration_mins: Option<u64>) -> String {
    match duration_mins {
        Some(WEEK_MINS) => "week".to_owned(),
        Some(mins) if mins > 0 && mins % (24 * 60) == 0 => format!("{}d", mins / (24 * 60)),
        Some(mins) if mins > 0 && mins % 60 == 0 => format!("{}h", mins / 60),
        Some(mins) => format!("{mins}m"),
        None => "window".to_owned(),
    }
}

/// Windows from an `account/rateLimits/read` result, labelled by their own length.
pub fn parse_codex_rate_limits(result: &Value) -> Result<Vec<QuotaWindow>, String> {
    let limits = result
        .get("rateLimits")
        .ok_or("response has no rateLimits")?;
    let windows: Vec<QuotaWindow> = ["primary", "secondary"]
        .iter()
        .filter_map(|slot| limits.get(*slot).filter(|window| !window.is_null()))
        .filter_map(|window| {
            Some(QuotaWindow {
                duration_mins: window.get("windowDurationMins").and_then(Value::as_u64),
                used_percent: window.get("usedPercent")?.as_f64()?,
                resets_at: window.get("resetsAt").and_then(Value::as_u64),
                model: None,
            })
        })
        .collect();
    if windows.is_empty() {
        return Err("response has no usage windows".to_owned());
    }
    Ok(windows)
}

/// Reads Codex usage and account by starting `executable app-server`; the executable
/// is the one discovery found, since a host started from a desktop launcher lacks the
/// shell's `PATH`.
pub fn read_codex(
    executable: &Path,
    timeout: Duration,
    now_secs: u64,
) -> (QuotaReport, Option<Account>) {
    let unavailable = |reason| QuotaReport::Unavailable {
        provider: Agent::Codex,
        reason,
    };
    match query_codex(executable, timeout) {
        Ok(answers) => {
            let account = answers.account.as_ref().and_then(parse_codex_account);
            let report = match answers
                .rate_limits
                .and_then(|result| parse_codex_rate_limits(&result))
            {
                Ok(windows) => QuotaReport::Available(QuotaReading {
                    provider: Agent::Codex,
                    windows,
                    read_at: now_secs,
                }),
                Err(reason) => unavailable(reason),
            };
            (report, account)
        }
        Err(reason) => (unavailable(reason), None),
    }
}

struct CodexAnswers {
    rate_limits: Result<Value, String>,
    /// `account/read`'s result, when it answered in time.
    account: Option<Value>,
}

const RATE_LIMITS_ID: u64 = 2;
const ACCOUNT_ID: u64 = 3;

fn query_codex(executable: &Path, timeout: Duration) -> Result<CodexAnswers, String> {
    let mut command = Command::new(executable);
    command
        .arg("app-server")
        .env("PATH", hq_discovery::shell::agent_path(executable))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for marker in crate::adapters::PARENT_SESSION_MARKERS {
        command.env_remove(marker);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("cannot start `codex app-server`: {error}"))?;
    let result = exchange_codex(&mut child, timeout);
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn exchange_codex(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<CodexAnswers, String> {
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let (lines_tx, lines_rx) = mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if lines_tx.send(line).is_err() {
                break;
            }
        }
    });

    let messages = [
        json!({"id": 1, "method": "initialize",
               "params": {"clientInfo": {"name": "hq-probe", "version": env!("CARGO_PKG_VERSION")}}}),
        json!({"method": "initialized"}),
        json!({"id": RATE_LIMITS_ID, "method": "account/rateLimits/read",
               "params": {"excludeResetCreditDetails": true}}),
        json!({"id": ACCOUNT_ID, "method": "account/read", "params": {}}),
    ];
    for message in messages {
        writeln!(stdin, "{message}").map_err(|error| format!("app-server write: {error}"))?;
    }
    stdin
        .flush()
        .map_err(|error| format!("app-server write: {error}"))?;

    let deadline = Instant::now() + timeout;
    let mut rate_limits = None;
    let mut account = None;
    while rate_limits.is_none() || account.is_none() {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let Ok(line) = lines_rx.recv_timeout(remaining) else {
            break;
        };
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        match message.get("id").and_then(Value::as_u64) {
            Some(RATE_LIMITS_ID) => rate_limits = Some(answer(&message, "account/rateLimits/read")),
            // A failed account read only leaves the account unknown.
            Some(ACCOUNT_ID) => account = Some(answer(&message, "account/read").ok()),
            _ => {}
        }
    }
    Ok(CodexAnswers {
        rate_limits: rate_limits
            .unwrap_or_else(|| Err("codex app-server did not answer in time".to_owned())),
        account: account.flatten(),
    })
}

/// A JSON-RPC answer's result, or its error as text.
fn answer(message: &Value, method: &str) -> Result<Value, String> {
    if let Some(error) = message.get("error") {
        let text = error
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or("unknown error");
        return Err(format!("{method} failed: {text}"));
    }
    message
        .get("result")
        .cloned()
        .ok_or_else(|| format!("{method} returned no result"))
}

/// The windows an agent always has, in minutes, for agents that leave a window out of
/// a report: Claude Code drops a window from its status line input once the window
/// resets, until it is used again.
pub fn standing_windows(agent: Agent) -> &'static [u64] {
    match agent {
        Agent::Claude => &[FIVE_HOURS_MINS, WEEK_MINS],
        Agent::Codex | Agent::Agy => &[],
    }
}

/// Windows from Claude Code status line input (`rate_limits.five_hour` and
/// `rate_limits.seven_day`). `None` when the input carries no limits.
pub fn parse_claude_status_line(input: &Value) -> Option<Vec<QuotaWindow>> {
    let limits = input.get("rate_limits")?;
    let windows: Vec<QuotaWindow> = [("five_hour", FIVE_HOURS_MINS), ("seven_day", WEEK_MINS)]
        .iter()
        .filter_map(|(key, mins)| {
            let window = limits.get(*key)?;
            Some(QuotaWindow {
                duration_mins: Some(*mins),
                used_percent: window.get("used_percentage")?.as_f64()?,
                resets_at: window.get("resets_at").and_then(Value::as_u64),
                model: None,
            })
        })
        .collect();
    (!windows.is_empty()).then_some(windows)
}

/// Replaces the snapshot atomically so readers never see a partial file.
pub fn save_snapshot(path: &Path, reading: &QuotaReading) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let partial = path.with_extension(format!("tmp{}", std::process::id()));
    std::fs::write(&partial, serde_json::to_vec(reading)?)?;
    std::fs::rename(&partial, path)
}

pub fn read_snapshot(path: &Path, provider: Agent) -> QuotaReport {
    let unavailable = |reason: &str| QuotaReport::Unavailable {
        provider,
        reason: reason.to_owned(),
    };
    let Ok(bytes) = std::fs::read(path) else {
        return unavailable("no session launched by Headquarter has reported limits yet");
    };
    match serde_json::from_slice::<QuotaReading>(&bytes) {
        Ok(reading) if reading.provider == provider => QuotaReport::Available(reading),
        Ok(_) => unavailable("snapshot belongs to another provider"),
        Err(_) => unavailable("snapshot is unreadable"),
    }
}

/// Copies a snapshot from an old location when the current one is missing.
pub fn migrate_snapshot(old: &Path, current: &Path) -> std::io::Result<bool> {
    if current.exists() || !old.exists() {
        return Ok(false);
    }
    if let Some(dir) = current.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::copy(old, current)?;
    Ok(true)
}

/// agy keeps its usage to its own status line, which Headquarter does not change; the
/// owner is told only that it is not shared (owner, 2026-10-23).
pub fn read_agy() -> QuotaReport {
    QuotaReport::Unavailable {
        provider: Agent::Agy,
        reason: "agy doesn't share its usage yet.".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_windows_are_labelled_by_duration() {
        let result = json!({"rateLimits": {
            "primary": {"usedPercent": 1, "windowDurationMins": 300, "resetsAt": 1792477739},
            "secondary": {"usedPercent": 20, "windowDurationMins": 10080, "resetsAt": 1792983466},
            "planType": "pro"
        }});
        let windows = parse_codex_rate_limits(&result).unwrap();
        assert_eq!(windows.len(), 2);
        assert_eq!(window_label(windows[0].duration_mins), "5h");
        assert_eq!(window_label(windows[1].duration_mins), "week");
        assert_eq!(window_label(Some(2 * 24 * 60)), "2d");
        assert_eq!(windows[1].used_percent, 20.0);
    }

    #[test]
    fn accounts_keep_only_their_kind_and_plan() {
        let codex = |account: Value| parse_codex_account(&json!({ "account": account }));
        assert_eq!(
            codex(json!({"type": "chatgpt", "email": "a@b.c", "planType": "pro"})),
            Some(Account::Subscription {
                plan: Some("pro".into())
            })
        );
        assert_eq!(codex(json!({"type": "apiKey"})), Some(Account::ApiKey));
        assert_eq!(codex(Value::Null), None);
        assert_eq!(codex(json!({"type": "somethingNew"})), None);

        let claude = |status: Value| parse_claude_auth_status(&status);
        assert_eq!(
            claude(
                json!({"loggedIn": true, "authMethod": "claude.ai", "apiProvider": "firstParty",
                          "email": "a@b.c", "subscriptionType": "max"})
            ),
            Some(Account::Subscription {
                plan: Some("max".into())
            })
        );
        assert_eq!(
            claude(json!({"loggedIn": true, "authMethod": "api_key", "apiProvider": "firstParty"})),
            Some(Account::ApiKey)
        );
        assert_eq!(
            claude(json!({"loggedIn": true, "authMethod": "none", "apiProvider": "bedrock"})),
            Some(Account::Cloud {
                provider: "bedrock".into()
            })
        );
        assert_eq!(
            claude(json!({"loggedIn": false, "authMethod": "claude.ai"})),
            None
        );
        let kept = serde_json::to_string(&Account::Subscription {
            plan: Some("max".into()),
        })
        .unwrap();
        assert_eq!(kept, r#"{"kind":"subscription","plan":"max"}"#);
    }

    #[test]
    fn claude_account_is_read_through_the_given_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("claude-auth");
        let stub = dir.join("claude");
        std::fs::write(
            &stub,
            "#!/bin/sh\n[ \"$1 $2 $3\" = 'auth status --json' ] || exit 3\necho '{\"loggedIn\":true,\"authMethod\":\"claude.ai\",\"apiProvider\":\"firstParty\",\"subscriptionType\":\"pro\"}'\n",
        )
        .unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            read_claude_account(&stub, Duration::from_secs(5)),
            Some(Account::Subscription {
                plan: Some("pro".into())
            })
        );
        assert_eq!(
            read_claude_account(&dir.join("missing"), Duration::from_secs(1)),
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn codex_without_windows_is_an_error() {
        assert!(parse_codex_rate_limits(&json!({})).is_err());
        assert!(parse_codex_rate_limits(&json!({"rateLimits": {"primary": null}})).is_err());
    }

    #[test]
    fn claude_status_line_limits() {
        let input = json!({"session_id": "s", "rate_limits": {
            "five_hour": {"used_percentage": 83.5, "resets_at": 1000},
            "seven_day": {"used_percentage": 64, "resets_at": 2000}
        }});
        let windows = parse_claude_status_line(&input).unwrap();
        assert_eq!(windows[0].duration_mins, Some(300));
        assert_eq!(windows[0].used_percent, 83.5);
        assert_eq!(windows[1].resets_at, Some(2000));
        assert_eq!(parse_claude_status_line(&json!({"session_id": "s"})), None);
    }

    #[test]
    fn staleness() {
        let window = QuotaWindow {
            duration_mins: Some(300),
            used_percent: 50.0,
            resets_at: Some(1000),
            model: None,
        };
        assert!(!window.is_stale(900, 999));
        assert!(window.is_stale(900, 1000));
        assert!(!window.is_stale(1000, 1500));
    }

    #[test]
    fn snapshot_round_trip_and_missing() {
        let path = std::env::temp_dir().join(format!("hq-quota-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&path);
        assert!(matches!(
            read_snapshot(&path, Agent::Claude),
            QuotaReport::Unavailable { .. }
        ));
        let reading = QuotaReading {
            provider: Agent::Claude,
            windows: vec![],
            read_at: 5,
        };
        save_snapshot(&path, &reading).unwrap();
        assert_eq!(
            read_snapshot(&path, Agent::Claude),
            QuotaReport::Available(reading)
        );
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn agy_is_unavailable_with_reason() {
        match read_agy() {
            QuotaReport::Unavailable { provider, reason } => {
                assert_eq!(provider, Agent::Agy);
                assert_eq!(reason, "agy doesn't share its usage yet.");
            }
            QuotaReport::Available(_) => panic!("agy quota should be unavailable"),
        }
    }

    fn scratch(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("hq-quota-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn codex_is_read_through_the_given_executable() {
        use std::os::unix::fs::PermissionsExt;
        let dir = scratch("stub");
        let stub = dir.join("my codex");
        std::fs::write(
            &stub,
            "#!/bin/sh\n[ \"$1\" = app-server ] || exit 3\nwhile read line; do case \"$line\" in *rateLimits*) echo '{\"id\":2,\"result\":{\"rateLimits\":{\"primary\":{\"usedPercent\":42,\"windowDurationMins\":300,\"resetsAt\":2000}}}}';; *account/read*) echo '{\"id\":3,\"result\":{\"account\":{\"type\":\"chatgpt\",\"email\":\"someone@example.com\",\"planType\":\"plus\"}}}';; esac; done\n",
        )
        .unwrap();
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
        let (report, account) = read_codex(&stub, Duration::from_secs(5), 1_000);
        assert_eq!(
            account,
            Some(Account::Subscription {
                plan: Some("plus".into())
            })
        );
        match report {
            QuotaReport::Available(reading) => {
                assert_eq!(reading.provider, Agent::Codex);
                assert_eq!(reading.read_at, 1_000);
                assert_eq!(reading.windows[0].used_percent, 42.0);
            }
            other => panic!("unexpected {other:?}"),
        }
        match read_codex(&dir.join("missing"), Duration::from_secs(1), 1_000) {
            (QuotaReport::Unavailable { reason, .. }, None) => {
                assert!(reason.contains("cannot start"), "{reason}");
            }
            other => panic!("unexpected {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn old_snapshot_is_copied_once() {
        let dir = scratch("migrate");
        let old = dir.join("probe/quota-claude.json");
        let current = dir.join("quota/claude.json");
        assert!(
            !migrate_snapshot(&old, &current).unwrap(),
            "nothing to copy"
        );
        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
        std::fs::write(&old, b"old").unwrap();
        assert!(migrate_snapshot(&old, &current).unwrap());
        assert_eq!(std::fs::read(&current).unwrap(), b"old");
        std::fs::write(&current, b"new").unwrap();
        assert!(!migrate_snapshot(&old, &current).unwrap(), "never replaces");
        assert_eq!(std::fs::read(&current).unwrap(), b"new");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
