//! Agents' usage readings, owned by the host so the app and `hq usage` show the same
//! numbers (`add-quota-footer` design D1). Readings follow activity
//! (`refresh-usage-on-activity` D1): Codex is read soon after a Codex turn ends and every
//! few minutes, Claude from its sessions' reports and from a hidden `/usage` read, agy from
//! its own headless `/usage` (add-agy-usage). Nothing is read while no one looks.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use hq_protocol::{AgentUsage, Frame, Response, UsageWindow};
use hq_signals::event::Agent;
use hq_signals::now_ms;
use hq_signals::quota::{self, Account, QuotaReading, QuotaReport};

use super::{Host, lock};
use crate::session::Outgoing;

/// Refresh requests closer together than this reuse the reading in progress or just
/// taken.
const REFRESH_FLOOR: Duration = Duration::from_secs(30);
/// The longest a refresh waits for its reads: the hidden `/usage` read's own limits
/// (start, screen, exit) with room to spare. Codex's limit is shorter.
const REFRESH_WAIT: Duration = Duration::from_secs(40);
/// How often the Claude snapshot file is checked while a client shows usage.
const CLAUDE_INTERVAL: Duration = Duration::from_secs(10);
const CODEX_TIMEOUT: Duration = Duration::from_secs(15);
/// How soon after a hosted Codex turn ends Codex is read; turns ending together share it.
const CODEX_AFTER_TURN: Duration = Duration::from_secs(10);
/// How soon a failed Codex read is tried again.
const CODEX_RETRY: Duration = Duration::from_secs(60);
/// What the owner sees when a provider does not report its usage: the provider's side,
/// and what to try. The technical reason goes to the host log.
const CODEX_UNREPORTED: &str =
    "Codex didn't report its usage. Open Codex once to check it is signed in, then refresh.";
const CLAUDE_UNREPORTED: &str = "Claude Code didn't report its usage. Open Claude Code once to check it is signed in, then refresh.";
const CLAUDE_TRUST: &str = "Claude Code asked whether to trust a folder before showing its usage. Open Claude Code once in one of your folders and answer, then refresh.";
/// How often the hidden `/usage` read runs while a client shows usage.
const CLAUDE_SCREEN_INTERVAL: Duration = Duration::from_secs(10 * 60);
/// A Claude turn in another terminal starts a hidden read at most this often.
const CLAUDE_ELSEWHERE_FLOOR: Duration = Duration::from_secs(2 * 60);
const CLAUDE_ACCOUNT_TIMEOUT: Duration = Duration::from_secs(10);
/// agy's `/usage` answered in about 8 s on 2026-10-23.
const AGY_TIMEOUT: Duration = Duration::from_secs(30);
/// How often agy is read while a client shows usage, as for Claude's hidden read.
const AGY_INTERVAL: Duration = Duration::from_secs(10 * 60);
/// How soon after a hosted agy turn ends agy is read; turns ending together share it.
const AGY_AFTER_TURN: Duration = Duration::from_secs(10);
const AGY_UNREPORTED: &str =
    "agy didn't report its usage. Open agy once to check it is signed in, then refresh.";
const LOOP: Duration = Duration::from_secs(1);

#[derive(Default)]
pub(super) struct UsageBook {
    /// The last good Codex reading.
    codex: Option<QuotaReading>,
    /// Why the last Codex read failed, if it did.
    codex_problem: Option<String>,
    codex_started: Option<Instant>,
    codex_reading: bool,
    claude_stamp: Option<SystemTime>,
    claude_checked: Option<Instant>,
    /// Accounts as last read, in memory only.
    codex_account: Option<Account>,
    claude_account: Option<Account>,
    claude_account_read: Option<Instant>,
    /// When a Codex read is due because a hosted Codex turn ended.
    codex_after_turn: Option<Instant>,
    /// The last good hidden `/usage` read, with per-model windows.
    claude_screen: Option<QuotaReading>,
    /// Why the last hidden read failed, if it did.
    claude_problem: Option<String>,
    claude_screen_started: Option<Instant>,
    claude_screen_reading: bool,
    /// The last good agy reading, and why the last read failed, if it did.
    agy: Option<QuotaReading>,
    agy_problem: Option<String>,
    agy_started: Option<Instant>,
    agy_reading: bool,
    /// When an agy read is due because a hosted agy turn ended.
    agy_after_turn: Option<Instant>,
}

impl Host {
    fn codex_file(&self) -> PathBuf {
        self.paths.dir.join("quota").join("codex.json")
    }

    fn agy_file(&self) -> PathBuf {
        self.paths.dir.join("quota").join("agy.json")
    }

    /// An empty folder of the host's own for agy's `/usage`, so the read touches no
    /// project of the owner's.
    fn agy_folder(&self) -> PathBuf {
        self.paths.dir.join("quota").join("agy-workspace")
    }

    /// Loads the last saved readings and moves the Claude snapshot from its Phase 0
    /// place, so a restart shows numbers at once with their age.
    pub(super) fn load_usage(&self) {
        if let (Some(old), Some(current)) = (
            hq_signals::helpers::legacy_claude_snapshot_path(),
            hq_signals::helpers::claude_snapshot_path(),
        ) && let Ok(true) = quota::migrate_snapshot(&old, &current)
        {
            self.log.line("usage snapshot moved agent=claude");
        }
        if let QuotaReport::Available(reading) =
            quota::read_snapshot(&self.codex_file(), Agent::Codex)
        {
            lock(&self.usage).codex = Some(reading);
        }
        if let QuotaReport::Available(reading) = quota::read_snapshot(&self.agy_file(), Agent::Agy)
        {
            lock(&self.usage).agy = Some(reading);
        }
    }

    pub(super) fn usage_loop(self: Arc<Self>) {
        loop {
            thread::sleep(LOOP);
            // Agents started before the login-shell PATH is known may not find `node`.
            if !self.has_usage_subscribers() || !self.login_path_settled.load(Ordering::SeqCst) {
                continue;
            }
            let (codex_due, claude_due, screen_due, agy_due) = {
                let mut book = lock(&self.usage);
                let after_turn = book
                    .codex_after_turn
                    .is_some_and(|due| Instant::now() >= due);
                if after_turn {
                    book.codex_after_turn = None;
                }
                let wait = if book.codex_problem.is_some() {
                    CODEX_RETRY.min(self.usage_interval)
                } else {
                    self.usage_interval
                };
                let codex_due = !book.codex_reading
                    && (after_turn
                        || book
                            .codex_started
                            .is_none_or(|started| started.elapsed() >= wait));
                let screen_due = !book.claude_screen_reading
                    && book
                        .claude_screen_started
                        .is_none_or(|started| started.elapsed() >= CLAUDE_SCREEN_INTERVAL);
                let claude_due = book
                    .claude_checked
                    .is_none_or(|checked| checked.elapsed() >= CLAUDE_INTERVAL);
                let agy_after_turn = book.agy_after_turn.is_some_and(|due| Instant::now() >= due);
                if agy_after_turn {
                    book.agy_after_turn = None;
                }
                let agy_due = !book.agy_reading
                    && (agy_after_turn
                        || book
                            .agy_started
                            .is_none_or(|started| started.elapsed() >= AGY_INTERVAL));
                (codex_due, claude_due, screen_due, agy_due)
            };
            if codex_due {
                self.start_codex_read();
            }
            if screen_due {
                self.start_claude_screen_read();
            }
            if agy_due {
                self.start_agy_read();
            }
            let account_changed = self.read_claude_account_when_due();
            if (claude_due && self.claude_snapshot_changed()) || account_changed {
                self.broadcast_usage();
            }
        }
    }

    fn has_usage_subscribers(&self) -> bool {
        lock(&self.subscribers)
            .iter()
            .any(|subscriber| subscriber.usage)
    }

    /// `Usage`: reads Codex first when it has never been read or the reading is due,
    /// so `hq usage` is current without an app open.
    pub(super) fn usage(&self) -> Vec<AgentUsage> {
        let due = {
            let book = lock(&self.usage);
            !book.codex_reading
                && book
                    .codex_started
                    .is_none_or(|started| started.elapsed() >= self.usage_interval)
        };
        if due && self.agent_wanted(Agent::Codex) {
            lock(&self.usage).codex_reading = true;
            self.read_codex_now();
            self.usage_settled.notify_all();
        }
        self.read_claude_account_when_due();
        self.usage_report()
    }

    /// `RefreshUsage`: reads again unless a read is under way or was just taken, and
    /// returns once no read is under way, so the answer means the readings are in;
    /// subscribers get the result.
    pub(super) fn refresh_usage(self: &Arc<Self>) {
        let recent = {
            let book = lock(&self.usage);
            book.codex_reading
                || book.codex_started.is_some_and(|started| {
                    started.elapsed() < REFRESH_FLOOR.min(self.usage_interval)
                })
        };
        self.claude_snapshot_changed();
        let screen_recent = {
            let book = lock(&self.usage);
            book.claude_screen_reading
                || book
                    .claude_screen_started
                    .is_some_and(|started| started.elapsed() < REFRESH_FLOOR)
        };
        if !screen_recent {
            self.start_claude_screen_read();
        }
        let agy_recent = {
            let book = lock(&self.usage);
            book.agy_reading
                || book
                    .agy_started
                    .is_some_and(|started| started.elapsed() < REFRESH_FLOOR)
        };
        if !agy_recent {
            self.start_agy_read();
        }
        if recent {
            self.broadcast_usage();
        } else {
            self.start_codex_read();
        }
        let book = lock(&self.usage);
        let _ = self
            .usage_settled
            .wait_timeout_while(book, REFRESH_WAIT, |book| {
                book.codex_reading || book.claude_screen_reading || book.agy_reading
            });
    }

    /// A hosted Codex session finished a turn: read Codex again shortly.
    pub(super) fn codex_turn_ended(&self) {
        let mut book = lock(&self.usage);
        if book.codex_after_turn.is_none() {
            book.codex_after_turn = Some(Instant::now() + CODEX_AFTER_TURN);
        }
    }

    /// A hosted agy session finished a turn: read agy again shortly.
    pub(super) fn agy_turn_ended(&self) {
        let mut book = lock(&self.usage);
        if book.agy_after_turn.is_none() {
            book.agy_after_turn = Some(Instant::now() + AGY_AFTER_TURN);
        }
    }

    fn start_agy_read(self: &Arc<Self>) {
        if !self.agent_wanted(Agent::Agy) {
            return;
        }
        {
            let mut book = lock(&self.usage);
            if book.agy_reading {
                return;
            }
            book.agy_reading = true;
            book.agy_started = Some(Instant::now());
        }
        let host = Arc::clone(self);
        thread::spawn(move || {
            host.read_agy_now();
            host.broadcast_usage();
            host.usage_settled.notify_all();
        });
    }

    /// Runs agy's `/usage`. A failure keeps the last good reading and records why. The
    /// caller has set `agy_reading`.
    fn read_agy_now(&self) {
        let folder = self.agy_folder();
        let report = match self.resolve_agent(Agent::Agy) {
            Ok(agy) => match std::fs::create_dir_all(&folder) {
                Ok(()) => quota::read_agy(&agy, &folder, AGY_TIMEOUT, now_ms() / 1000),
                Err(error) => QuotaReport::Unavailable {
                    provider: Agent::Agy,
                    reason: format!("cannot make {}: {error}", folder.display()),
                },
            },
            Err(error) => {
                self.log.line(&format!(
                    "usage read agent=agy failed reason={}",
                    problem_text(*error)
                ));
                // Nothing to open or sign in to: the plain line for usage that cannot
                // be read at all.
                let mut book = lock(&self.usage);
                book.agy_reading = false;
                book.agy_problem = Some(quota::not_available_text(Agent::Agy));
                return;
            }
        };
        let mut book = lock(&self.usage);
        book.agy_reading = false;
        match report {
            QuotaReport::Available(reading) => {
                if let Err(error) = quota::save_snapshot(&self.agy_file(), &reading) {
                    self.log
                        .line(&format!("usage snapshot not saved agent=agy error={error}"));
                }
                book.agy = Some(reading);
                book.agy_problem = None;
            }
            QuotaReport::Unavailable { reason, .. } => {
                self.log
                    .line(&format!("usage read agent=agy failed reason={reason}"));
                book.agy_problem = Some(AGY_UNREPORTED.to_owned());
            }
        }
        self.log.line(&format!(
            "usage read agent=agy ok={}",
            book.agy_problem.is_none()
        ));
    }

    /// A Claude session in another terminal finished a turn: its usage changed, so read
    /// `/usage` again unless that happened recently.
    pub(super) fn claude_turn_ended_elsewhere(self: &Arc<Self>) {
        if !self.has_usage_subscribers() {
            return;
        }
        let recent = lock(&self.usage)
            .claude_screen_started
            .is_some_and(|started| started.elapsed() < CLAUDE_ELSEWHERE_FLOOR);
        if !recent {
            self.start_claude_screen_read();
        }
    }

    fn start_claude_screen_read(self: &Arc<Self>) {
        if !self.agent_wanted(Agent::Claude) {
            return;
        }
        {
            let mut book = lock(&self.usage);
            if book.claude_screen_reading {
                return;
            }
            book.claude_screen_reading = true;
            book.claude_screen_started = Some(Instant::now());
        }
        let host = Arc::clone(self);
        thread::spawn(move || {
            host.read_claude_screen_now();
            host.broadcast_usage();
            host.usage_settled.notify_all();
        });
    }

    /// Runs the hidden `/usage` read. A failure keeps the last good reading and records
    /// why. The caller has set `claude_screen_reading`.
    fn read_claude_screen_now(&self) {
        let outcome = self
            .resolve_agent(Agent::Claude)
            .map_err(|error| problem_text(*error))
            .and_then(|claude| {
                let folder = self
                    .trusted_claude_folder()
                    .ok_or("no folder Claude Code was used in is known yet")?;
                super::claude_usage::read_usage_screen(&claude, &folder)
            });
        let mut book = lock(&self.usage);
        book.claude_screen_reading = false;
        match outcome {
            Ok(windows) => {
                book.claude_screen = Some(QuotaReading {
                    provider: Agent::Claude,
                    windows,
                    read_at: now_ms() / 1000,
                });
                book.claude_problem = None;
            }
            Err(reason) => {
                self.log
                    .line(&format!("usage read agent=claude failed reason={reason}"));
                // A trust question the owner can answer says so; anything else is
                // Claude Code not reporting.
                let shown = if reason == super::claude_usage::TRUST_ASKED {
                    CLAUDE_TRUST
                } else {
                    CLAUDE_UNREPORTED
                };
                book.claude_problem = Some(shown.to_owned());
            }
        }
        self.log.line(&format!(
            "usage read agent=claude source=usage-screen ok={}",
            book.claude_problem.is_none()
        ));
    }

    /// The folder of the most recent Claude conversation that still exists: Claude has
    /// been trusted there, so its trust question does not appear.
    fn trusted_claude_folder(&self) -> Option<PathBuf> {
        let mut rows: Vec<_> = lock(&self.store)
            .list()
            .ok()?
            .into_iter()
            .filter(|row| row.agent == Agent::Claude)
            .collect();
        rows.sort_by_key(|row| std::cmp::Reverse(row.last_activity_at));
        rows.into_iter()
            .map(|row| PathBuf::from(row.cwd))
            .find(|folder| folder.is_dir())
    }

    fn start_codex_read(self: &Arc<Self>) {
        if !self.agent_wanted(Agent::Codex) {
            return;
        }
        {
            let mut book = lock(&self.usage);
            if book.codex_reading {
                return;
            }
            book.codex_reading = true;
        }
        let host = Arc::clone(self);
        thread::spawn(move || {
            host.read_codex_now();
            host.broadcast_usage();
            host.usage_settled.notify_all();
        });
    }

    /// Reads Codex through the discovered executable. A failure keeps the last good
    /// reading and records why. The caller has set `codex_reading`.
    fn read_codex_now(&self) {
        lock(&self.usage).codex_started = Some(Instant::now());
        let (report, account) = match self.resolve_agent(Agent::Codex) {
            Ok(path) => match quota::read_codex(&path, CODEX_TIMEOUT, now_ms() / 1000) {
                (QuotaReport::Unavailable { provider, reason }, account) => {
                    self.log
                        .line(&format!("usage read agent=codex failed reason={reason}"));
                    let reason = CODEX_UNREPORTED.to_owned();
                    (QuotaReport::Unavailable { provider, reason }, account)
                }
                answered => answered,
            },
            Err(error) => (
                QuotaReport::Unavailable {
                    provider: Agent::Codex,
                    reason: problem_text(*error),
                },
                None,
            ),
        };
        let mut book = lock(&self.usage);
        book.codex_reading = false;
        book.codex_account = account;
        match report {
            QuotaReport::Available(reading) => {
                if let Err(error) = quota::save_snapshot(&self.codex_file(), &reading) {
                    self.log.line(&format!(
                        "usage snapshot not saved agent=codex error={error}"
                    ));
                }
                book.codex = Some(reading);
                book.codex_problem = None;
            }
            QuotaReport::Unavailable { reason, .. } => book.codex_problem = Some(reason),
        }
        self.log.line(&format!(
            "usage read agent=codex ok={}",
            book.codex_problem.is_none()
        ));
    }

    /// Reads Claude's account when it was never read or the usage interval passed;
    /// returns whether it changed.
    fn read_claude_account_when_due(&self) -> bool {
        let due = lock(&self.usage)
            .claude_account_read
            .is_none_or(|read| read.elapsed() >= self.usage_interval);
        if !due || !self.agent_wanted(Agent::Claude) {
            return false;
        }
        lock(&self.usage).claude_account_read = Some(Instant::now());
        let account = self
            .resolve_agent(Agent::Claude)
            .ok()
            .and_then(|path| quota::read_claude_account(&path, CLAUDE_ACCOUNT_TIMEOUT));
        let mut book = lock(&self.usage);
        let changed = book.claude_account != account;
        book.claude_account = account;
        changed
    }

    /// Whether the Claude snapshot changed since the last check.
    fn claude_snapshot_changed(&self) -> bool {
        let stamp = hq_signals::helpers::claude_snapshot_path()
            .and_then(|path| std::fs::metadata(path).ok())
            .and_then(|metadata| metadata.modified().ok());
        let mut book = lock(&self.usage);
        book.claude_checked = Some(Instant::now());
        let changed = book.claude_stamp != stamp;
        book.claude_stamp = stamp;
        changed
    }

    /// Agents turned off in the settings file are left out of usage.
    fn agent_wanted(&self, agent: Agent) -> bool {
        lock(&self.settings)
            .current()
            .map_or(true, |settings| settings.agent_enabled(agent.id()))
    }

    fn usage_report(&self) -> Vec<AgentUsage> {
        let mut agents = Vec::new();
        for agent in Agent::ALL {
            if !self.agent_wanted(agent) {
                continue;
            }
            let usage = match agent {
                Agent::Codex => {
                    let book = lock(&self.usage);
                    let mut usage = from_reading(agent, book.codex.as_ref());
                    usage.problem.clone_from(&book.codex_problem);
                    usage.account.clone_from(&book.codex_account);
                    if usage.read_at.is_none() && usage.problem.is_none() {
                        usage.problem = Some("not read yet".to_owned());
                    }
                    usage
                }
                Agent::Claude => {
                    let report = match hq_signals::helpers::claude_snapshot_path() {
                        Some(path) => quota::read_snapshot(&path, Agent::Claude),
                        None => QuotaReport::Unavailable {
                            provider: agent,
                            reason: "the data folder is unknown".to_owned(),
                        },
                    };
                    let book = lock(&self.usage);
                    let mut usage = claude_usage(
                        report,
                        book.claude_screen.as_ref(),
                        book.claude_problem.as_deref(),
                    );
                    usage.account.clone_from(&book.claude_account);
                    usage
                }
                Agent::Agy => {
                    let book = lock(&self.usage);
                    let mut usage = from_reading(agent, book.agy.as_ref());
                    usage.problem.clone_from(&book.agy_problem);
                    if usage.read_at.is_none() && usage.problem.is_none() {
                        usage.problem = Some("not read yet".to_owned());
                    }
                    usage
                }
            };
            agents.push(usage);
        }
        agents
    }

    pub(super) fn broadcast_usage(&self) {
        let frame = Frame::response(&Response::UsageChanged {
            agents: self.usage_report(),
        });
        lock(&self.subscribers).retain(|subscriber| {
            !subscriber.usage || subscriber.tx.send(Outgoing::Frame(frame.clone())).is_ok()
        });
    }
}

fn from_reading(agent: Agent, reading: Option<&QuotaReading>) -> AgentUsage {
    AgentUsage {
        agent,
        windows: reading
            .map(|reading| {
                reading
                    .windows
                    .iter()
                    .map(|window| UsageWindow {
                        minutes: window.duration_mins,
                        used_percent: window.used_percent,
                        resets_at: window.resets_at,
                        model: window.model.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        read_at: reading.map(|reading| reading.read_at),
        problem: None,
        account: None,
    }
}

fn from_report(agent: Agent, report: QuotaReport) -> AgentUsage {
    match report {
        QuotaReport::Available(reading) => from_reading(agent, Some(&reading)),
        QuotaReport::Unavailable { reason, .. } => AgentUsage {
            problem: Some(reason),
            ..from_reading(agent, None)
        },
    }
}

/// Claude's usage from its two sources: a hosted session's status line report and the
/// hidden `/usage` read. The newer one gives the five-hour and weekly numbers; per-model
/// windows come only from `/usage`. `problem` is why the last `/usage` read failed.
fn claude_usage(
    report: QuotaReport,
    screen: Option<&QuotaReading>,
    problem: Option<&str>,
) -> AgentUsage {
    let status_line = match report {
        QuotaReport::Available(reading) => Some(reading),
        QuotaReport::Unavailable { reason, .. } => {
            if screen.is_none() {
                let mut usage = from_report(
                    Agent::Claude,
                    QuotaReport::Unavailable {
                        provider: Agent::Claude,
                        reason,
                    },
                );
                if let Some(problem) = problem {
                    usage.problem = Some(problem.to_owned());
                }
                return usage;
            }
            None
        }
    };
    let newest = match (&status_line, screen) {
        (Some(line), Some(screen)) if line.read_at > screen.read_at => line.clone(),
        (_, Some(screen)) => screen.clone(),
        (Some(line), None) => line.clone(),
        (None, None) => unreachable!("handled above"),
    };
    let mut reading = QuotaReading {
        windows: newest
            .windows
            .iter()
            .filter(|window| window.model.is_none())
            .cloned()
            .collect(),
        ..newest
    };
    if let Some(screen) = screen {
        reading.windows.extend(
            screen
                .windows
                .iter()
                .filter(|window| window.model.is_some())
                .cloned(),
        );
    }
    let mut usage = from_reading(Agent::Claude, Some(&reading));
    usage.problem = problem.map(str::to_owned);
    usage
}

pub(super) fn problem_text(response: Response) -> String {
    match response {
        Response::Error { message, .. } => message,
        other => format!("{other:?}"),
    }
}
