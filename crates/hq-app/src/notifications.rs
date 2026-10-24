//! When the app tells the owner about a session through the system's notifications
//! (add-notifications D1, D2). Only changes notify: a session must already be known,
//! be in the sidebar, and not be the one the owner is watching.

use hq_config::Notifications;
use hq_protocol::SessionInfo;
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
    let agent = agent_name(current.agent);
    let wants = match current.status {
        Status::NeedsYou if settings.needs_you => asks(current.needs_reason),
        Status::DoneUnread if settings.replied => "replied",
        _ => return None,
    };
    Some(Notice {
        tag: current.id.clone(),
        // What the sidebar row shows: the owner's name for it, else the agent's title,
        // else "Untitled conversation · id" (owner, 2026-10-24).
        title: hq_protocol::sidebar::display_title(current),
        body: format!("{agent} {wants}"),
    })
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
        let mut before = titled.clone();
        before.status = Status::Working;
        let posted = notice(Some(&before), &titled, false, Notifications::default()).unwrap();
        assert_eq!(
            posted.title, "Fixing Unresponsive Docker Daemon",
            "not the id"
        );
    }

    #[test]
    fn a_finished_turn_notifies_as_replied_without_the_folder() {
        let posted = change(Status::Working, Status::DoneUnread).unwrap();
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
        let watched = notice(
            Some(&working),
            &session(Status::DoneUnread),
            true,
            Notifications::default(),
        );
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
        let replied = notice(
            Some(&working),
            &session(Status::DoneUnread),
            false,
            quiet_replies,
        );
        assert_eq!(replied, None);
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
        let replied = change(Status::Working, Status::DoneUnread).unwrap();
        assert_eq!(asked.tag, replied.tag);
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
