//! Session details for the app's header (`openspec/changes/add-session-header/design.md`,
//! D5): what each agent reports, kept in memory per session and sent only to clients
//! following that session. Nothing here is written to the index or the log.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use hq_protocol::{Frame, Response};
use hq_signals::details::{CodexRollout, SessionDetails, codex_rollout, git_branch};
use hq_signals::event::Agent;

use super::{Host, lock};
use crate::session::Outgoing;

/// How often followed sessions' branches and Codex rollouts are read again.
const DETAILS_INTERVAL: Duration = Duration::from_secs(2);

/// What the host knows about sessions' details.
#[derive(Default)]
pub(super) struct DetailsBook {
    /// What agents reported themselves (Claude's status line), by session id.
    reported: HashMap<String, SessionDetails>,
    /// Followed Codex rollouts, by session id.
    rollouts: HashMap<String, CodexRollout>,
    /// The details last sent for each session, so unchanged ones are not sent again.
    sent: HashMap<String, SessionDetails>,
}

impl Host {
    pub(super) fn details_loop(self: Arc<Self>) {
        loop {
            thread::sleep(DETAILS_INTERVAL);
            for session in self.followed_sessions() {
                self.send_details(&session, false);
            }
        }
    }

    /// Details an agent reported for a hosted session (Claude's status line); checked
    /// again here because they arrive over the hooks socket.
    pub(super) fn details_reported(&self, session: &str, raw: &str) {
        if !lock(&self.live).contains_key(session) {
            return;
        }
        let Ok(details) = serde_json::from_str::<SessionDetails>(raw) else {
            self.log.line("details payload unreadable");
            return;
        };
        lock(&self.details)
            .reported
            .insert(session.to_owned(), details.sanitized());
        self.send_details(session, false);
    }

    /// A client now shows `session` (or nothing): it gets that session's details at
    /// once and whenever they change.
    pub(super) fn follow_details(&self, connection: u64, session: Option<String>) {
        for subscriber in lock(&self.subscribers).iter_mut() {
            if subscriber.connection == connection {
                subscriber.follows.clone_from(&session);
            }
        }
        if let Some(session) = session {
            self.send_details(&session, true);
        }
    }

    /// The session ended: its details are dropped.
    pub(super) fn forget_details(&self, session: &str) {
        let mut book = lock(&self.details);
        book.reported.remove(session);
        book.rollouts.remove(session);
        book.sent.remove(session);
    }

    fn followed_sessions(&self) -> Vec<String> {
        let mut sessions: Vec<String> = lock(&self.subscribers)
            .iter()
            .filter_map(|subscriber| subscriber.follows.clone())
            .collect();
        sessions.sort();
        sessions.dedup();
        sessions
    }

    /// Gathers the session's details and sends them to its followers when they changed,
    /// or always with `force` (a client just started following).
    fn send_details(&self, session: &str, force: bool) {
        let Ok(info) = self.info(session) else {
            return;
        };
        let conversation = info.agent_session_id.clone();
        let details = {
            let mut book = lock(&self.details);
            let mut details = match info.agent {
                Agent::Codex => codex_details(&mut book, session, conversation.as_deref()),
                Agent::Claude | Agent::Agy => {
                    book.reported.get(session).cloned().unwrap_or_default()
                }
            };
            if details.branch.is_none() {
                details.branch = git_branch(Path::new(&info.cwd));
            }
            if !force && book.sent.get(session) == Some(&details) {
                return;
            }
            book.sent.insert(session.to_owned(), details.clone());
            details
        };
        let frame = Frame::response(&Response::DetailsChanged {
            session: session.to_owned(),
            details,
        });
        lock(&self.subscribers).retain(|subscriber| {
            subscriber.follows.as_deref() != Some(session)
                || subscriber.tx.send(Outgoing::Frame(frame.clone())).is_ok()
        });
    }
}

/// A Codex session's details from its rollout file, once its conversation is known.
fn codex_details(
    book: &mut DetailsBook,
    session: &str,
    conversation: Option<&str>,
) -> SessionDetails {
    if !book.rollouts.contains_key(session) {
        let Some(path) = conversation.and_then(|conversation| {
            hq_signals::paths::codex_home().and_then(|home| codex_rollout(&home, conversation))
        }) else {
            return SessionDetails::default();
        };
        book.rollouts
            .insert(session.to_owned(), CodexRollout::new(path));
    }
    let rollout = book
        .rollouts
        .get_mut(session)
        .expect("inserted above when missing");
    rollout.poll();
    rollout.details().clone()
}
