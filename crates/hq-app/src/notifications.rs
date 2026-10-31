//! When the app tells the owner about a session through the system's notifications
//! (add-notifications D1, D2). Only changes notify: a session must already be known,
//! be in the sidebar, and not be the one the owner is watching.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use hq_config::Notifications;
use hq_protocol::SessionInfo;
use hq_protocol::sidebar::{self, Beside};
use hq_signals::event::{Agent, NeedsReason};
use hq_signals::status::Status;

/// A notification to post: the session's name over what its agent wants (owner,
/// 2026-10-22). `tag` is the session id, so a newer one replaces the older.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notice {
    pub tag: String,
    pub title: String,
    pub body: String,
}

/// The notification for a session that changed from `previous` to `current`, if any.
/// `watched` is true while it is shown in the focused pane of the active window.
pub fn notice(
    previous: Option<&SessionInfo>,
    current: &SessionInfo,
    watched: bool,
    settings: Notifications,
) -> Option<Notice> {
    let previous = previous?;
    // One running in another terminal is that terminal's to report (ext-rows-without-status).
    if !current.in_sidebar
        || current.external.is_some()
        || watched
        || previous.status == current.status
    {
        return None;
    }
    // A reply waits until the session settles (`PendingReplies`).
    match current.status {
        Status::NeedsYou if settings.needs_you => Some(posted(current, asks(current.needs_reason))),
        _ => None,
    }
}

/// The "replied" notification for a session whose reply has settled, unless the owner
/// is watching it, it left the sidebar, or replies are turned off.
pub fn replied(current: &SessionInfo, watched: bool, settings: Notifications) -> Option<Notice> {
    let shown = current.in_sidebar && current.external.is_none() && !watched;
    (shown && settings.replied && current.status == Status::DoneUnread)
        .then(|| posted(current, "replied"))
}

fn posted(current: &SessionInfo, wants: &str) -> Notice {
    Notice {
        tag: current.id.clone(),
        // What the sidebar row shows: the owner's name for it, else the agent's title,
        // else "Untitled conversation · id" (owner, 2026-10-24).
        title: sidebar::display_title(current),
        body: format!("{} {wants}", agent_name(current.agent)),
    }
}

/// How long a reply must stay unread, with no background work, before it is announced.
/// Of the owner's turns that Claude Code started again by itself, most that came quickly
/// came within 10 s (fix-false-done D1).
pub const REPLY_QUIET: Duration = Duration::from_secs(10);

/// Replies waiting to be announced (fix-false-done D1): each session's quiet-since time,
/// `None` while its background work runs.
#[derive(Debug, Default)]
pub struct PendingReplies {
    quiet_since: HashMap<String, Option<Instant>>,
}

impl PendingReplies {
    /// Follows a session's change: a new reply starts waiting; work, needs you, reading it
    /// or leaving the sidebar drops it; background work pauses the wait until it ends.
    pub fn track(
        &mut self,
        previous: Option<&SessionInfo>,
        current: &SessionInfo,
        settings: Notifications,
        now: Instant,
    ) {
        let unread = current.status == Status::DoneUnread
            && current.in_sidebar
            && current.external.is_none();
        if !unread {
            self.quiet_since.remove(&current.id);
            return;
        }
        let busy = sidebar::beside(current) == Some(Beside::Background);
        let quiet = (!busy).then_some(now);
        let new_reply = previous.is_some_and(|previous| previous.status != Status::DoneUnread);
        if new_reply && settings.replied {
            self.quiet_since.insert(current.id.clone(), quiet);
        } else if let Some(since) = self.quiet_since.get_mut(&current.id) {
            if busy {
                *since = None;
            } else if since.is_none() {
                *since = quiet;
            }
        }
    }

    /// The sessions whose replies have been quiet long enough; they stop waiting.
    pub fn due(&mut self, now: Instant) -> Vec<String> {
        let due: Vec<String> = self
            .quiet_since
            .iter()
            .filter(|(_, since)| since.is_some_and(|since| now >= since + REPLY_QUIET))
            .map(|(id, _)| id.clone())
            .collect();
        for id in &due {
            self.quiet_since.remove(id);
        }
        due
    }

    pub fn is_empty(&self) -> bool {
        self.quiet_since.is_empty()
    }
}

/// The notifications for sidebar sessions the host could not start with itself that
/// failed after `told_until` (Unix milliseconds), the latest failure already told, so
/// each is told once even across app restarts (add-inactive-sessions D6).
pub fn start_failures(sessions: &[SessionInfo], told_until: u64) -> Vec<Notice> {
    sessions
        .iter()
        .filter(|session| session.in_sidebar)
        .filter_map(|session| {
            let error = session.start_error.as_ref()?;
            (error.at > told_until).then(|| Notice {
                tag: session.id.clone(),
                title: hq_protocol::sidebar::display_title(session),
                body: format!("Couldn't start: {}", error.reason),
            })
        })
        .collect()
}

/// The session a clicked notification opens: the one its tag names, while it is still
/// in the sidebar (D3).
pub fn clicked<'a>(sessions: &'a [SessionInfo], tag: &str) -> Option<&'a SessionInfo> {
    sessions
        .iter()
        .find(|session| session.id == tag && session.in_sidebar)
}

fn agent_name(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "Claude",
        Agent::Codex => "Codex",
        Agent::Agy => "Antigravity",
    }
}

fn asks(reason: Option<NeedsReason>) -> &'static str {
    match reason {
        Some(NeedsReason::Permission) => "needs your permission",
        Some(NeedsReason::Question) => "has a question",
        Some(NeedsReason::Auth) => "needs you to sign in",
        Some(NeedsReason::Input) => "is waiting for you",
        Some(NeedsReason::Unknown) | None => "needs you",
    }
}

#[cfg(test)]
mod tests {
    use hq_protocol::{External, Origin, StartError};
    use hq_signals::event::Agent;

    use super::*;

    fn session(status: Status) -> SessionInfo {
        SessionInfo {
            id: "s-1".into(),
            agent: Agent::Claude,
            cwd: "/home/me/api".into(),
            name: "Fix login".into(),
            agent_session_id: None,
            status,
            needs_reason: None,
            created_at: 0,
            last_activity_at: 0,
            ended_at: None,
            live: true,
            origin: Origin::Hosted,
            title: Some("the agent's own title".into()),
            external: None,
            in_sidebar: true,
            group: None,
            owner_named: true,
            starting: false,
            start_error: None,
            offered: true,
            phone: None,
            background: Default::default(),
            review: false,
        }
    }

    fn change(from: Status, to: Status) -> Option<Notice> {
        notice(
            Some(&session(from)),
            &session(to),
            false,
            Notifications::default(),
        )
    }

    #[test]
    fn needing_the_owner_names_the_session_agent_and_reason() {
        let mut asking = session(Status::NeedsYou);
        asking.needs_reason = Some(NeedsReason::Permission);
        let posted = notice(
            Some(&session(Status::Working)),
            &asking,
            false,
            Notifications::default(),
        );
        assert_eq!(
            posted,
            Some(Notice {
                tag: "s-1".into(),
                title: "Fix login".into(),
                body: "Claude needs your permission".into(),
            })
        );
        assert_eq!(asks(Some(NeedsReason::Question)), "has a question");
        assert_eq!(asks(None), "needs you");
    }

    #[test]
    fn a_session_running_elsewhere_never_notifies() {
        let mut asking = session(Status::NeedsYou);
        asking.external = Some(External {
            pid: Some(7),
            status: Some(Status::NeedsYou),
            background: false,
        });
        let posted = notice(
            Some(&session(Status::Working)),
            &asking,
            false,
            Notifications::default(),
        );
        assert_eq!(posted, None);
    }

    #[test]
    fn the_title_is_what_the_sidebar_shows() {
        let mut titled = session(Status::DoneUnread);
        titled.owner_named = false;
        titled.name = "claude-e0c6f139".into();
        titled.title = Some("Fixing Unresponsive Docker Daemon".into());
        let posted = replied(&titled, false, Notifications::default()).unwrap();
        assert_eq!(
            posted.title, "Fixing Unresponsive Docker Daemon",
            "not the id"
        );
    }

    #[test]
    fn a_finished_turn_notifies_as_replied_without_the_folder() {
        assert_eq!(
            change(Status::Working, Status::DoneUnread),
            None,
            "a reply waits to settle"
        );
        let posted = replied(
            &session(Status::DoneUnread),
            false,
            Notifications::default(),
        )
        .unwrap();
        assert_eq!(posted.title, "Fix login", "the owner's name for it wins");
        assert_eq!(posted.body, "Claude replied");
        assert!(!posted.body.contains("api"), "no folder");
    }

    #[test]
    fn other_changes_and_repeats_stay_quiet() {
        assert_eq!(change(Status::NeedsYou, Status::Working), None);
        assert_eq!(change(Status::DoneUnread, Status::Done), None);
        assert_eq!(change(Status::Done, Status::Inactive), None);
        assert_eq!(change(Status::NeedsYou, Status::NeedsYou), None);
    }

    #[test]
    fn the_first_listing_never_notifies() {
        let posted = notice(
            None,
            &session(Status::NeedsYou),
            false,
            Notifications::default(),
        );
        assert_eq!(posted, None);
    }

    #[test]
    fn watched_and_unlisted_sessions_stay_quiet() {
        let working = session(Status::Working);
        let watched = replied(&session(Status::DoneUnread), true, Notifications::default());
        assert_eq!(watched, None);
        let mut unlisted = session(Status::NeedsYou);
        unlisted.in_sidebar = false;
        let posted = notice(Some(&working), &unlisted, false, Notifications::default());
        assert_eq!(posted, None);
    }

    #[test]
    fn each_kind_can_be_turned_off() {
        let quiet_replies = Notifications {
            needs_you: true,
            replied: false,
        };
        let working = session(Status::Working);
        let reply = replied(&session(Status::DoneUnread), false, quiet_replies);
        assert_eq!(reply, None);
        let needs_you = notice(
            Some(&working),
            &session(Status::NeedsYou),
            false,
            quiet_replies,
        );
        assert!(needs_you.is_some());
        let quiet_asks = Notifications {
            needs_you: false,
            replied: true,
        };
        let asked = notice(
            Some(&working),
            &session(Status::NeedsYou),
            false,
            quiet_asks,
        );
        assert_eq!(asked, None);
    }

    #[test]
    fn a_click_opens_the_tagged_sidebar_session_only() {
        let mut other = session(Status::Done);
        other.id = "s-2".into();
        let mut removed = session(Status::Done);
        removed.id = "s-3".into();
        removed.in_sidebar = false;
        let sessions = [session(Status::NeedsYou), other, removed];
        assert_eq!(
            clicked(&sessions, "s-2").map(|s| s.id.as_str()),
            Some("s-2")
        );
        assert_eq!(clicked(&sessions, "s-3"), None, "left the sidebar since");
        assert_eq!(clicked(&sessions, "gone"), None);
    }

    #[test]
    fn a_session_keeps_one_tag_so_newer_notices_replace_older() {
        let asked = change(Status::Working, Status::NeedsYou).unwrap();
        let reply = replied(
            &session(Status::DoneUnread),
            false,
            Notifications::default(),
        )
        .unwrap();
        assert_eq!(asked.tag, reply.tag);
    }

    fn pending_after(steps: &[(SessionInfo, u64)]) -> (PendingReplies, Instant) {
        let start = Instant::now();
        let mut pending = PendingReplies::default();
        let mut previous: Option<SessionInfo> = None;
        for (current, at) in steps {
            let now = start + Duration::from_secs(*at);
            pending.track(previous.as_ref(), current, Notifications::default(), now);
            previous = Some(current.clone());
        }
        (pending, start)
    }

    fn at(start: Instant, seconds: u64) -> Instant {
        start + Duration::from_secs(seconds)
    }

    #[test]
    fn a_reply_is_announced_after_ten_quiet_seconds() {
        let (mut pending, start) = pending_after(&[
            (session(Status::Working), 0),
            (session(Status::DoneUnread), 1),
        ]);
        assert!(pending.due(at(start, 10)).is_empty());
        assert_eq!(pending.due(at(start, 11)), ["s-1"]);
        assert!(pending.is_empty());
    }

    #[test]
    fn carrying_on_reading_or_needing_the_owner_drops_the_reply() {
        for next in [Status::Working, Status::Done, Status::NeedsYou] {
            let (mut pending, start) = pending_after(&[
                (session(Status::Working), 0),
                (session(Status::DoneUnread), 1),
                (session(next), 5),
            ]);
            assert!(pending.due(at(start, 60)).is_empty(), "{next:?}");
        }
    }

    #[test]
    fn background_work_holds_the_reply_until_it_ends() {
        let mut busy = session(Status::DoneUnread);
        busy.background.agents = 1;
        let (mut pending, start) = pending_after(&[
            (session(Status::Working), 0),
            (busy.clone(), 1),
            (busy, 30),
            (session(Status::DoneUnread), 40),
        ]);
        assert!(
            pending.due(at(start, 45)).is_empty(),
            "quiet only since the work ended"
        );
        assert_eq!(pending.due(at(start, 50)), ["s-1"]);
    }

    #[test]
    fn replies_turned_off_never_wait() {
        let mut pending = PendingReplies::default();
        let quiet = Notifications {
            needs_you: true,
            replied: false,
        };
        let now = Instant::now();
        pending.track(
            Some(&session(Status::Working)),
            &session(Status::DoneUnread),
            quiet,
            now,
        );
        assert!(pending.is_empty());
        pending.track(
            None,
            &session(Status::DoneUnread),
            Notifications::default(),
            now,
        );
        assert!(pending.is_empty(), "the first listing never notifies");
    }

    #[test]
    fn a_start_failure_is_told_once() {
        let mut failed = session(Status::Inactive);
        failed.start_error = Some(StartError {
            reason: "folder `/home/me/api` does not exist".into(),
            at: 2_000,
        });
        let mut outside = failed.clone();
        outside.id = "s-2".into();
        outside.in_sidebar = false;
        let sessions = [failed, outside];
        assert_eq!(
            start_failures(&sessions, 1_000),
            [Notice {
                tag: "s-1".into(),
                title: "Fix login".into(),
                body: "Couldn't start: folder `/home/me/api` does not exist".into(),
            }]
        );
        assert!(start_failures(&sessions, 2_000).is_empty(), "told already");
    }
}
