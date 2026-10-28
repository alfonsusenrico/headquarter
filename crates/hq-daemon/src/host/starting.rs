//! Sessions that start with the host (add-inactive-sessions D3): every sidebar session
//! that was running when the host last stopped starts again on its conversation, one at
//! a time in sidebar order. A client may start one ahead of its turn. Each counts as
//! starting from the host's start until its agent first reports, so the owner sees them
//! settle one by one.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use hq_protocol::{Response, SessionInfo, StartError, sidebar};
use hq_signals::now_ms;

use super::{Host, lock};

/// The longest wait for a started agent's first report before the next one starts.
const READY_WAIT: Duration = Duration::from_secs(5);
/// The longest wait for the first outside scan, which says which conversations already
/// run in another terminal.
const OUTSIDE_WAIT: Duration = Duration::from_secs(10);

#[derive(Default)]
pub(super) struct Starts {
    /// Sessions still to start, or started and not yet ready.
    pending: Mutex<HashSet<String>>,
    settled: Condvar,
    /// Why a session could not start with the host; in memory only (D6).
    errors: Mutex<HashMap<String, StartError>>,
}

impl Host {
    /// The sidebar sessions marked to start with the host, in sidebar order, now counted
    /// as starting. Called before the host serves clients, so their first list shows it.
    pub(super) fn queue_starts(&self) -> Vec<String> {
        let rows = match lock(&self.store).list() {
            Ok(rows) => rows,
            Err(error) => {
                self.log
                    .line(&format!("start queue unavailable error={error}"));
                return Vec::new();
            }
        };
        let groups = match self.groups() {
            Ok(groups) => groups,
            Err(error) => {
                self.log
                    .line(&format!("start queue without groups error={error:?}"));
                Vec::new()
            }
        };
        let marked: Vec<SessionInfo> = rows
            .into_iter()
            .filter(|row| row.sidebar_added_at.is_some() && row.interrupted_at.is_some())
            .map(|row| self.describe(row))
            .collect();
        let order = start_order(&marked, &groups);
        lock(&self.starts.pending).extend(order.iter().cloned());
        order
    }

    /// Starts `order` one at a time; runs on its own thread.
    pub(super) fn start_with_host(self: &Arc<Self>, order: &[String]) {
        if order.is_empty() {
            return;
        }
        self.wait_for_outside_scan();
        for id in order {
            if self.stopping.load(std::sync::atomic::Ordering::SeqCst) {
                // Unstarted sessions keep their mark and start with the next host.
                return;
            }
            if !lock(&self.starts.pending).contains(id) {
                continue;
            }
            if lock(&self.live).contains_key(id) {
                self.wait_until_ready(id);
                continue;
            }
            if lock(&self.outside).linked.contains_key(id) {
                self.skip_start(id, "running in another terminal");
                continue;
            }
            match self.resume(id, None) {
                Ok(_) => self.wait_until_ready(id),
                Err(error) => {
                    let reason = match *error {
                        Response::Error { message, .. } => message,
                        other => format!("{other:?}"),
                    };
                    self.fail_start(id, reason);
                }
            }
        }
    }

    fn wait_for_outside_scan(&self) {
        let deadline = Instant::now() + OUTSIDE_WAIT;
        while !lock(&self.outside).scanned && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(100));
        }
    }

    /// Waits for the session's first report, or `READY_WAIT`, then counts it as started.
    fn wait_until_ready(&self, id: &str) {
        let deadline = Instant::now() + READY_WAIT;
        let mut pending = lock(&self.starts.pending);
        while pending.contains(id) {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                break;
            }
            pending = self
                .starts
                .settled
                .wait_timeout(pending, left)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
        let was_pending = pending.remove(id);
        drop(pending);
        if was_pending {
            self.broadcast(id);
        }
    }

    /// The session will not start with the host, and that is no failure.
    fn skip_start(&self, id: &str, why: &str) {
        if let Err(error) = lock(&self.store).clear_start_mark(id) {
            self.log
                .line(&format!("index update failed id={id} error={error}"));
        }
        self.settle_start(id);
        self.log
            .line(&format!("start skipped id={id} reason={why}"));
    }

    /// The session could not start with the host: it stays inactive with the reason.
    pub(super) fn fail_start(&self, id: &str, reason: String) {
        if let Err(error) = lock(&self.store).clear_start_mark(id) {
            self.log
                .line(&format!("index update failed id={id} error={error}"));
        }
        self.log.line(&format!("start failed id={id}"));
        lock(&self.starts.errors).insert(
            id.to_owned(),
            StartError {
                reason,
                at: now_ms(),
            },
        );
        self.settle_start(id);
    }

    /// The session no longer counts as starting: its agent reported, ended, or the
    /// session left the sidebar.
    pub(super) fn settle_start(&self, id: &str) {
        let was_pending = lock(&self.starts.pending).remove(id);
        if was_pending {
            self.starts.settled.notify_all();
            self.broadcast(id);
        }
    }

    pub(super) fn is_starting(&self, id: &str) -> bool {
        lock(&self.starts.pending).contains(id)
    }

    pub(super) fn start_error(&self, id: &str) -> Option<StartError> {
        lock(&self.starts.errors).get(id).cloned()
    }

    /// Forgets why a session could not start: it started, or left the sidebar.
    pub(super) fn clear_start_error(&self, id: &str) {
        lock(&self.starts.errors).remove(id);
    }
}

/// `sessions` in the order the sidebar shows them: each group in the owner's order, then
/// the ungrouped ones.
fn start_order(sessions: &[SessionInfo], groups: &[hq_protocol::GroupInfo]) -> Vec<String> {
    let layout = sidebar::layout(sessions, groups, None);
    layout
        .groups
        .iter()
        .flat_map(|section| section.sessions.iter())
        .chain(layout.ungrouped.iter())
        .map(|session| session.id.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use hq_protocol::{GroupInfo, Origin};
    use hq_signals::event::Agent;
    use hq_signals::status::Status;

    use super::*;

    fn session(id: &str, group: Option<i64>, last_activity_at: u64) -> SessionInfo {
        SessionInfo {
            id: id.to_owned(),
            agent: Agent::Claude,
            cwd: "/w".to_owned(),
            name: id.to_owned(),
            agent_session_id: None,
            status: Status::Inactive,
            needs_reason: None,
            created_at: 0,
            last_activity_at,
            ended_at: None,
            live: false,
            origin: Origin::Hosted,
            title: None,
            external: None,
            in_sidebar: true,
            group,
            owner_named: false,
            starting: false,
            start_error: None,
            offered: true,
            phone: None,
        }
    }

    #[test]
    fn sessions_start_in_the_order_the_sidebar_shows_them() {
        let groups = [
            GroupInfo {
                id: 2,
                name: "Web".into(),
                collapsed: false,
                color: None,
            },
            GroupInfo {
                id: 1,
                name: "Backend".into(),
                collapsed: true,
                color: None,
            },
        ];
        let sessions = [
            session("loose", None, 900),
            session("backend-old", Some(1), 100),
            session("web", Some(2), 50),
            session("backend-new", Some(1), 500),
        ];
        assert_eq!(
            start_order(&sessions, &groups),
            ["web", "backend-new", "backend-old", "loose"]
        );
    }
}
