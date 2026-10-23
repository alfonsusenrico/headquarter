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
    if !current.in_sidebar || watched || previous.status == current.status {
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
        title: current.name.clone(),
        body: format!("{agent} {wants}"),
    })
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
    use hq_protocol::{External, Origin};
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
            interrupted: false,
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
    fn a_finished_turn_notifies_as_replied_without_the_folder() {
        let posted = change(Status::Working, Status::DoneUnread).unwrap();
        assert_eq!(
            posted.title, "Fix login",
            "the name, never the agent's title"
        );
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
    fn outside_sessions_notify_like_hosted_ones() {
        let mut outside = session(Status::NeedsYou);
        outside.origin = Origin::History;
        outside.external = Some(External {
            pid: Some(42),
            status: Some(Status::NeedsYou),
        });
        let mut before = outside.clone();
        before.status = Status::Working;
        let posted = notice(Some(&before), &outside, false, Notifications::default());
        assert_eq!(posted.unwrap().body, "Claude needs you");
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
}
