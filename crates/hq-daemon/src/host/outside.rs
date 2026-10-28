//! Agents running in other terminals: observed every few seconds and linked to their
//! conversation's row. Observe only: nothing here signals or touches those processes,
//! and none of it is stored.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant, SystemTime};

use hq_protocol::{External, Origin, SessionInfo};
use hq_signals::event::Agent;
use hq_signals::external::{self, ExternalSession};
use hq_signals::paths;
use hq_signals::status::Status;
use hq_signals::subagents::{self, RunningSubagent};

use super::{Host, lock};

/// The scan's pace while a client shows the sessions or a hosted session works.
const OUTSIDE_INTERVAL: Duration = Duration::from_secs(5);
/// The scan's pace while nobody looks and nothing hosted works (pace-outside-scan).
const UNSEEN_INTERVAL: Duration = Duration::from_secs(60);
/// How often `claude agents --json` is started for Claude Code's background agents when
/// none of them changed; running sessions come from Claude Code's records every scan.
const CLAUDE_BACKGROUND_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Default)]
pub(super) struct OutsideView {
    /// Row id → the outside process running that row's conversation.
    pub linked: HashMap<String, External>,
    /// Outside agents no known conversation matches, listed while they run.
    pub unlinked: Vec<SessionInfo>,
    /// Whether a scan has finished since the host started.
    pub scanned: bool,
    /// Source failures last reported, so the log records changes, not every scan.
    failures: HashSet<String>,
    /// Row id → Codex sub-agents still running for that hosted session.
    pub subagents: HashMap<String, Vec<RunningSubagent>>,
    /// Claude Code processes in other terminals that were busy at the last scan.
    claude_busy: HashSet<u32>,
    /// When `claude agents --json` was last started, and the background agents it
    /// listed then.
    claude_asked: Option<(Instant, SystemTime)>,
    claude_background: Vec<ExternalSession>,
}

/// An agy conversation as agy's own index lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct AgyConversation {
    pub folder: String,
    pub id: String,
    /// Unix milliseconds.
    pub last_activity_at: u64,
}

/// Asks the scan loop for a scan at the fast pace now: a client started showing the
/// sessions or listed them.
#[derive(Default)]
pub(super) struct OutsideWake {
    asked: Mutex<bool>,
    signal: Condvar,
}

/// How old a scan may grow before the next one runs.
fn scan_pace(someone_looks: bool, hosted_working: bool) -> Duration {
    if someone_looks || hosted_working {
        OUTSIDE_INTERVAL
    } else {
        UNSEEN_INTERVAL
    }
}

impl Host {
    /// Checks every `OUTSIDE_INTERVAL` whether a scan is due at the current pace, so a
    /// change of pace takes effect within that time without other code waking it.
    pub(super) fn outside_loop(self: Arc<Self>) {
        let mut memory = external::ScanMemory::default();
        let mut scanned_at: Option<Instant> = None;
        let mut asked = false;
        loop {
            let pace = if asked {
                OUTSIDE_INTERVAL
            } else {
                let hosted_working = lock(&self.live)
                    .values()
                    .any(|hosted| hosted.status() == Status::Working);
                scan_pace(self.someone_looks(), hosted_working)
            };
            if scanned_at.is_none_or(|at| at.elapsed() >= pace) {
                scanned_at = Some(Instant::now());
                self.refresh_outside(&mut memory);
            }
            asked = self.wait_for_ask();
        }
    }

    /// Waits up to `OUTSIDE_INTERVAL` for a client to ask for a scan; true if one did.
    fn wait_for_ask(&self) -> bool {
        let mut asked = lock(&self.outside_wake.asked);
        if !*asked {
            asked = self
                .outside_wake
                .signal
                .wait_timeout(asked, OUTSIDE_INTERVAL)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
        std::mem::take(&mut *asked)
    }

    /// A subscribed client shows the sessions.
    fn someone_looks(&self) -> bool {
        lock(&self.subscribers)
            .iter()
            .any(|subscriber| subscriber.visible)
    }

    /// Asks for a scan at once unless the last one is younger than `OUTSIDE_INTERVAL`.
    /// Never waits for it: a scan can take seconds when an agent is slow to answer.
    pub(super) fn wake_outside(&self) {
        *lock(&self.outside_wake.asked) = true;
        self.outside_wake.signal.notify_one();
    }

    fn refresh_outside(self: &Arc<Self>, memory: &mut external::ScanMemory) {
        let mut exclude: Vec<u32> = lock(&self.live)
            .values()
            .filter_map(|hosted| hosted.pid)
            .collect();
        exclude.push(std::process::id());
        let claude = self.locate_claude();
        let asked = lock(&self.outside).claude_asked;
        let ask_claude = match asked {
            None => true,
            Some((at, wall)) => {
                at.elapsed() >= CLAUDE_BACKGROUND_INTERVAL
                    || external::claude_background_changed_since(wall)
            }
        };
        let mut report = external::scan(memory, &exclude, claude.as_deref(), ask_claude);
        {
            let mut view = lock(&self.outside);
            if ask_claude {
                // Recorded even when the command failed, so a failing one is retried
                // on the slow interval rather than every scan.
                view.claude_asked = Some((Instant::now(), SystemTime::now()));
            }
            match report.claude_background.take() {
                Some(background) => view.claude_background = background,
                None => report
                    .sessions
                    .extend(view.claude_background.iter().cloned()),
            }
        }
        self.refresh_background(&report.hosted_claude);
        self.notice_claude_turns_elsewhere(&report.sessions);
        let agy_conversations = lock(&self.agy_conversations).clone();
        let agy_folders: Vec<&str> = report
            .sessions
            .iter()
            .filter(|session| session.agent == Agent::Agy)
            .filter_map(|session| session.cwd.as_deref())
            .collect();
        let shared_folders: HashSet<String> = agy_folders
            .iter()
            .filter(|folder| agy_folders.iter().filter(|other| other == folder).count() > 1)
            .map(|folder| (*folder).to_owned())
            .collect();

        let mut linked = HashMap::new();
        let mut unlinked = Vec::new();
        for session in report.sessions {
            let row = conversation_of(&session, &agy_conversations, &shared_folders).and_then(
                |conversation| {
                    lock(&self.store)
                        .find_by_conversation(session.agent, &conversation)
                        .ok()
                        .flatten()
                },
            );
            let external = External {
                pid: session.pid,
                status: session.status,
                background: session.background,
            };
            match row {
                Some(row) => {
                    linked.insert(row.id, external);
                }
                None => unlinked.push(unlinked_entry(&session, external)),
            }
        }

        let failures: HashSet<String> = report
            .failures
            .iter()
            .map(|failure| format!("{}: {}", failure.source, failure.error))
            .collect();
        let (changed_rows, unlinked_news) = {
            let mut view = lock(&self.outside);
            for failure in failures.difference(&view.failures) {
                self.log
                    .line(&format!("outside scan source failed: {failure}"));
            }
            let changed = linked
                .iter()
                .filter(|(id, external)| view.linked.get(*id) != Some(*external))
                .map(|(id, _)| id.clone())
                .chain(
                    view.linked
                        .keys()
                        .filter(|id| !linked.contains_key(*id))
                        .cloned(),
                )
                .collect::<Vec<String>>();
            let unlinked_news = unlinked_changes(&view.unlinked, &unlinked);
            view.linked = linked;
            view.unlinked = unlinked;
            view.failures = failures;
            view.scanned = true;
            (changed, unlinked_news)
        };
        for id in changed_rows {
            self.broadcast(&id);
        }
        for session in unlinked_news {
            self.send_change(session);
        }
    }
}

impl Host {
    /// A Claude session in another terminal that was busy and is no longer has finished
    /// a turn, which changed the owner's usage.
    fn notice_claude_turns_elsewhere(self: &Arc<Self>, sessions: &[ExternalSession]) {
        let busy: HashSet<u32> = sessions
            .iter()
            .filter(|session| {
                session.agent == Agent::Claude && session.status == Some(Status::Working)
            })
            .filter_map(|session| session.pid)
            .collect();
        let finished = {
            let mut view = lock(&self.outside);
            let finished = view.claude_busy.difference(&busy).next().is_some();
            view.claude_busy = busy;
            finished
        };
        if finished {
            self.claude_turn_ended_elsewhere();
        }
    }

    /// Marks the hosted sessions whose agent runs work for them in the background:
    /// Claude Code reporting itself busy after its turn, or Codex sub-agents still
    /// running. Hosted Claude agents are matched by the process they run under.
    fn refresh_background(&self, hosted_claude: &[(u32, Status)]) {
        let live: Vec<_> = lock(&self.live).values().cloned().collect();
        let codex_threads: Vec<(String, String)> = live
            .iter()
            .filter(|hosted| hosted.agent == Agent::Codex)
            .filter_map(|hosted| {
                let row = lock(&self.store).get(&hosted.id).ok().flatten()?;
                Some((hosted.id.clone(), row.agent_session_id?))
            })
            .collect();
        let parents: Vec<String> = codex_threads
            .iter()
            .map(|(_, thread)| thread.clone())
            .collect();
        let running = paths::codex_home()
            .map(|home| subagents::running_subagents(&home.join("sessions"), &parents))
            .unwrap_or_default();
        let by_row: HashMap<String, Vec<RunningSubagent>> = codex_threads
            .into_iter()
            .filter_map(|(row, thread)| Some((row, running.get(&thread)?.clone())))
            .collect();
        for hosted in &live {
            let busy = match hosted.agent {
                Agent::Claude => hosted
                    .pid
                    .is_some_and(|pid| hosted_claude.contains(&(pid, Status::Working))),
                Agent::Codex => by_row.contains_key(&hosted.id),
                Agent::Agy => false,
            };
            hosted.set_background(busy);
            self.refresh(hosted);
        }
        lock(&self.outside).subagents = by_row;
    }
}

/// The conversation an outside process runs, when it can be known. Claude and Codex
/// report it. agy names it only when started with `--conversation`; otherwise it is
/// the conversation in its folder written most recently since the process started,
/// unless other agy processes share that folder (fix-agy-link, 2026-10-23: agy's busy
/// flag could read idle while it waited for the owner, which dropped the link).
fn conversation_of(
    session: &ExternalSession,
    agy: &[AgyConversation],
    shared_folders: &HashSet<String>,
) -> Option<String> {
    if session.session_id.is_some() || session.agent != Agent::Agy {
        return session.session_id.clone();
    }
    let cwd = session.cwd.as_ref()?;
    if shared_folders.contains(cwd) {
        return None;
    }
    let since = session.started_at?.saturating_mul(1000);
    agy.iter()
        .filter(|conversation| {
            &conversation.folder == cwd && conversation.last_activity_at >= since
        })
        .max_by_key(|conversation| conversation.last_activity_at)
        .map(|conversation| conversation.id.clone())
}

/// What clients must hear about agents with no known conversation since the last scan:
/// new or changed entries, and gone ones (linked or exited) as no longer offered, so no
/// client keeps an entry from its first list (add-sessions-running-elsewhere D4).
fn unlinked_changes(before: &[SessionInfo], now: &[SessionInfo]) -> Vec<SessionInfo> {
    let mut changes: Vec<SessionInfo> = now
        .iter()
        .filter(|entry| !before.contains(entry))
        .cloned()
        .collect();
    changes.extend(
        before
            .iter()
            .filter(|old| !now.iter().any(|entry| entry.id == old.id))
            .map(|old| SessionInfo {
                offered: false,
                ..old.clone()
            }),
    );
    changes
}

fn unlinked_entry(session: &ExternalSession, external: External) -> SessionInfo {
    let pid = session
        .pid
        .map_or_else(|| "unknown".to_owned(), |pid| pid.to_string());
    let seen_at = session
        .last_activity_at
        .or(session.started_at)
        .map_or(0, |secs| secs * 1000);
    SessionInfo {
        id: format!("external-{}-{pid}", session.agent),
        agent: session.agent,
        cwd: session.cwd.clone().unwrap_or_default(),
        name: format!("{} pid {pid}", session.agent),
        agent_session_id: session.session_id.clone(),
        // Not tracked by the index; the live state is in `external`.
        status: Status::Inactive,
        needs_reason: None,
        created_at: session.started_at.map_or(0, |secs| secs * 1000),
        last_activity_at: seen_at,
        ended_at: None,
        live: false,
        origin: Origin::External,
        title: None,
        external: Some(external),
        in_sidebar: false,
        group: None,
        owner_named: false,
        starting: false,
        start_error: None,
        offered: true,
        phone: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(agent: Agent, cwd: &str, session_id: Option<&str>) -> ExternalSession {
        ExternalSession {
            agent,
            pid: Some(42),
            cwd: Some(cwd.to_owned()),
            session_id: session_id.map(str::to_owned),
            started_at: Some(1),
            last_activity_at: None,
            status: None,
            background: false,
            background_id: None,
        }
    }

    #[test]
    fn the_scan_slows_only_when_nobody_looks_and_nothing_hosted_works() {
        assert_eq!(scan_pace(true, false), OUTSIDE_INTERVAL);
        assert_eq!(scan_pace(false, true), OUTSIDE_INTERVAL);
        assert_eq!(scan_pace(true, true), OUTSIDE_INTERVAL);
        assert_eq!(scan_pace(false, false), UNSEEN_INTERVAL);
    }

    #[test]
    fn claude_and_codex_link_by_their_reported_id() {
        let none = HashSet::new();
        assert_eq!(
            conversation_of(&session(Agent::Claude, "/w", Some("c1")), &[], &none),
            Some("c1".into())
        );
        assert_eq!(
            conversation_of(&session(Agent::Codex, "/w", None), &[], &none),
            None
        );
    }

    fn agy_conversation(folder: &str, id: &str, last_activity_at: u64) -> AgyConversation {
        AgyConversation {
            folder: folder.to_owned(),
            id: id.to_owned(),
            last_activity_at,
        }
    }

    #[test]
    fn agy_links_to_the_conversation_its_folder_wrote_since_it_started() {
        // The process started at 1 s; "old" was last written before, "new" since.
        let conversations = vec![
            agy_conversation("/w", "old", 500),
            agy_conversation("/w", "new", 9_000),
            agy_conversation("/w", "newer", 9_500),
            agy_conversation("/x", "elsewhere", 9_000),
        ];
        let none = HashSet::new();
        assert_eq!(
            conversation_of(&session(Agent::Agy, "/w", None), &conversations, &none),
            Some("newer".into()),
            "the one written most recently"
        );
        assert_eq!(
            conversation_of(&session(Agent::Agy, "/y", None), &conversations, &none),
            None,
            "nothing written in its folder since it started"
        );
        let shared = HashSet::from(["/w".to_owned()]);
        assert_eq!(
            conversation_of(&session(Agent::Agy, "/w", None), &conversations, &shared),
            None,
            "two agy processes in one folder are not told apart by time"
        );
        assert_eq!(
            conversation_of(
                &session(Agent::Agy, "/w", Some("old")),
                &conversations,
                &shared
            ),
            Some("old".into()),
            "a conversation named on its command line links anyway"
        );
    }

    #[test]
    fn clients_hear_unlinked_agents_appear_change_and_go() {
        let external = |status| External {
            pid: Some(7),
            status,
            background: false,
        };
        let waiting = unlinked_entry(
            &session(Agent::Claude, "/w", None),
            external(Some(Status::NeedsYou)),
        );
        let other = unlinked_entry(
            &ExternalSession {
                pid: Some(8),
                ..session(Agent::Claude, "/w", None)
            },
            external(None),
        );
        let only = std::slice::from_ref;
        assert_eq!(unlinked_changes(&[], only(&waiting)), only(&waiting));
        assert!(
            unlinked_changes(only(&waiting), only(&waiting)).is_empty(),
            "nothing to say when nothing changed"
        );
        let working = SessionInfo {
            external: Some(external(Some(Status::Working))),
            ..waiting.clone()
        };
        assert_eq!(
            unlinked_changes(only(&waiting), only(&working)),
            only(&working)
        );
        let gone = unlinked_changes(&[working.clone(), other.clone()], only(&other));
        assert_eq!(gone.len(), 1);
        assert_eq!(gone[0].id, working.id);
        assert!(!gone[0].offered, "linked or exited: no longer offered");
    }

    #[test]
    fn unlinked_entries_are_marked_external() {
        let entry = unlinked_entry(
            &session(Agent::Agy, "/w", None),
            External {
                pid: Some(42),
                status: None,
                background: false,
            },
        );
        assert_eq!(entry.origin, Origin::External);
        assert_eq!(entry.id, "external-agy-42");
        assert_eq!(entry.external.unwrap().pid, Some(42));
    }
}
