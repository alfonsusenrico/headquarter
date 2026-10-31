//! What the host told the phone about the computer's sessions, and home as the owner
//! chose it (add-android-app D7): alerts first, then the owner's groups, then sessions in
//! no group, arranged by the desktop's own `hq_protocol::sidebar`.

use hq_protocol::{AgentUsage, GroupInfo, SessionInfo, sidebar, usage};
use hq_signals::event::NeedsReason;
use hq_signals::status::Status;

/// A session as a home row.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Row {
    pub id: String,
    /// Display only: never stored on the phone.
    pub title: String,
    /// `claude`, `codex` or `agy`.
    pub agent: String,
    pub status: RowStatus,
    /// The age in the desktop's style (`0m`, `3h`, `2d`).
    pub age: String,
    /// The folder's label, as the desktop shows it.
    pub folder: String,
    /// What an alert row says under the title, such as "Claude needs your permission".
    pub detail: String,
    /// Background work or a plan to review for a session at its prompt, shown in place of
    /// the age (add-background-work-status D4).
    pub beside: Option<RowBeside>,
    /// What runs in the background or waits for review, for the session's header.
    pub beside_detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RowStatus {
    Working,
    Starting,
    NeedsYou,
    Unread,
    Idle,
    Inactive,
    /// Running in another terminal on the computer: that terminal answers it.
    Elsewhere,
}

/// What a session at its prompt still has going: "background" or "review".
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum RowBeside {
    Background,
    Review,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct GroupRows {
    pub id: i64,
    pub name: String,
    /// The colour's palette name (`mauve`, `teal`...), the picked or automatic one.
    pub color: String,
    pub rows: Vec<Row>,
    /// Something in it needs the owner, has an unread reply or works: the group opens by
    /// itself.
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct UsageMeter {
    pub agent: String,
    /// The window's name, such as `5h`.
    pub window: String,
    /// Percent used, 0 to 100.
    pub percent: f64,
}

#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct Home {
    pub alerts: Vec<Row>,
    pub groups: Vec<GroupRows>,
    pub ungrouped: Vec<Row>,
    pub usage: Vec<UsageMeter>,
}

/// A past conversation Add sessions offers.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PastConversation {
    pub id: String,
    pub title: String,
    pub agent: String,
    pub age: String,
    pub folder: String,
}

#[derive(Debug, Default)]
pub(crate) struct Mirror {
    pub sessions: Vec<SessionInfo>,
    pub groups: Vec<GroupInfo>,
    pub usage: Vec<AgentUsage>,
}

impl Mirror {
    pub fn session_changed(&mut self, session: SessionInfo) {
        match self
            .sessions
            .iter_mut()
            .find(|known| known.id == session.id)
        {
            Some(known) => *known = session,
            None => self.sessions.push(session),
        }
    }

    pub fn session(&self, id: &str) -> Option<&SessionInfo> {
        self.sessions.iter().find(|session| session.id == id)
    }

    pub fn home(&self, now_ms: u64) -> Home {
        let layout = sidebar::layout(&self.sessions, &self.groups, None);
        let row = |session: &SessionInfo| Row {
            id: session.id.clone(),
            title: sidebar::display_title(session),
            agent: session.agent.id().to_owned(),
            status: row_status(session),
            age: sidebar::age(now_ms.saturating_sub(session.last_activity_at)),
            folder: layout.folder(session).to_owned(),
            detail: detail(session),
            beside: sidebar::beside(session).map(|beside| match beside {
                sidebar::Beside::Background => RowBeside::Background,
                sidebar::Beside::Review => RowBeside::Review,
            }),
            beside_detail: sidebar::beside_detail(session),
        };
        // Needs you, longest waiting first (the desktop's order), then unread replies,
        // newest first.
        let mut alerts: Vec<Row> = layout
            .needs_you
            .iter()
            .map(|session| row(session))
            .collect();
        let mut unread: Vec<&SessionInfo> = self
            .sessions
            .iter()
            .filter(|session| {
                // A reply whose background work still runs stays in its group, showing
                // "background", until the work ends (fix-false-done D3).
                session.in_sidebar
                    && session.external.is_none()
                    && session.status == Status::DoneUnread
                    && sidebar::beside(session) != Some(sidebar::Beside::Background)
            })
            .collect();
        unread.sort_by_key(|session| std::cmp::Reverse(session.last_activity_at));
        alerts.extend(unread.into_iter().map(row));
        let groups = layout
            .groups
            .iter()
            .map(|section| {
                let rows: Vec<Row> = section
                    .sessions
                    .iter()
                    .map(|session| row(session))
                    .collect();
                GroupRows {
                    id: section.group.id,
                    name: section.group.name.clone(),
                    color: section.group.shown_color().name().to_owned(),
                    active: rows.iter().any(|row| {
                        row.beside.is_some()
                            || matches!(
                                row.status,
                                RowStatus::NeedsYou | RowStatus::Unread | RowStatus::Working
                            )
                    }),
                    rows,
                }
            })
            .collect();
        let now_secs = now_ms / 1000;
        let usage = self
            .usage
            .iter()
            .filter_map(|agent| {
                let window = usage::tightest(agent, now_secs)?;
                Some(UsageMeter {
                    agent: agent.agent.id().to_owned(),
                    window: usage::window_name(window),
                    percent: window.used_percent.clamp(0.0, 100.0),
                })
            })
            .collect();
        Home {
            alerts,
            groups,
            ungrouped: layout
                .ungrouped
                .iter()
                .map(|session| row(session))
                .collect(),
            usage,
        }
    }

    /// Past conversations not in the sidebar that Add sessions offers, newest first.
    pub fn past(&self, now_ms: u64) -> Vec<PastConversation> {
        let mut past: Vec<&SessionInfo> = self
            .sessions
            .iter()
            .filter(|session| !session.in_sidebar && session.offered && session.external.is_none())
            .collect();
        past.sort_by_key(|session| std::cmp::Reverse(session.last_activity_at));
        let mut paths: Vec<&str> = past.iter().map(|session| session.cwd.as_str()).collect();
        paths.sort_unstable();
        paths.dedup();
        let labels = hq_protocol::group_labels(&paths);
        past.into_iter()
            .map(|session| PastConversation {
                id: session.id.clone(),
                title: sidebar::display_title(session),
                agent: session.agent.id().to_owned(),
                age: sidebar::age(now_ms.saturating_sub(session.last_activity_at)),
                folder: labels
                    .get(session.cwd.as_str())
                    .cloned()
                    .unwrap_or_else(|| session.cwd.clone()),
            })
            .collect()
    }
}

fn row_status(session: &SessionInfo) -> RowStatus {
    if session.external.is_some() {
        return RowStatus::Elsewhere;
    }
    if session.starting {
        return RowStatus::Starting;
    }
    match session.status {
        Status::Working => RowStatus::Working,
        Status::NeedsYou => RowStatus::NeedsYou,
        Status::DoneUnread => RowStatus::Unread,
        Status::Done => RowStatus::Idle,
        Status::Inactive => RowStatus::Inactive,
    }
}

/// What an alert row says: who waits and why.
fn detail(session: &SessionInfo) -> String {
    let agent = agent_name(session);
    match (session.status, session.needs_reason) {
        (Status::NeedsYou, Some(NeedsReason::Permission)) => {
            format!("{agent} needs your permission")
        }
        (Status::NeedsYou, Some(NeedsReason::Question)) => format!("{agent} has a question"),
        (Status::NeedsYou, Some(NeedsReason::Auth)) => format!("{agent} needs you to sign in"),
        (Status::NeedsYou, _) => format!("{agent} needs you"),
        (Status::DoneUnread, _) => format!("{agent} replied"),
        (status, _) => format!("{agent} · {}", status.label()),
    }
}

fn agent_name(session: &SessionInfo) -> &'static str {
    match session.agent.id() {
        "claude" => "Claude",
        "codex" => "Codex",
        _ => "Antigravity",
    }
}

#[cfg(test)]
mod tests {
    use hq_protocol::Origin;
    use hq_signals::event::Agent;

    use super::*;

    fn session(id: &str, status: Status, at: u64, group: Option<i64>) -> SessionInfo {
        SessionInfo {
            id: id.to_owned(),
            agent: Agent::Claude,
            cwd: "/work/api".to_owned(),
            name: id.to_owned(),
            agent_session_id: None,
            status,
            needs_reason: None,
            created_at: 0,
            last_activity_at: at,
            ended_at: None,
            live: status != Status::Inactive,
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
            background: Default::default(),
            review: false,
        }
    }

    fn group(id: i64, name: &str) -> GroupInfo {
        GroupInfo {
            id,
            name: name.to_owned(),
            collapsed: false,
            color: None,
        }
    }

    #[test]
    fn alerts_come_first_then_groups_in_order_then_the_rest() {
        let mut waiting = session("waiting", Status::NeedsYou, 1_000, Some(2));
        waiting.needs_reason = Some(NeedsReason::Permission);
        let mirror = Mirror {
            sessions: vec![
                session("old-reply", Status::DoneUnread, 2_000, None),
                session("new-reply", Status::DoneUnread, 3_000, Some(1)),
                waiting,
                session("busy", Status::Working, 4_000, Some(1)),
                session("quiet", Status::Done, 5_000, Some(2)),
                session("asleep", Status::Inactive, 500, None),
            ],
            groups: vec![group(1, "Backend"), group(2, "Website")],
            usage: Vec::new(),
        };
        let home = mirror.home(3_600_000);
        let alerts: Vec<&str> = home.alerts.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(alerts, ["waiting", "new-reply", "old-reply"]);
        assert_eq!(home.alerts[0].detail, "Claude needs your permission");
        assert_eq!(home.alerts[1].detail, "Claude replied");
        let names: Vec<&str> = home
            .groups
            .iter()
            .map(|group| group.name.as_str())
            .collect();
        assert_eq!(names, ["Backend", "Website"]);
        let backend: Vec<&str> = home.groups[0]
            .rows
            .iter()
            .map(|row| row.id.as_str())
            .collect();
        assert_eq!(backend, ["busy", "new-reply"]);
        assert!(home.groups[0].active && home.groups[1].active);
        let rest: Vec<&str> = home.ungrouped.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(rest, ["old-reply", "asleep"]);
        assert_eq!(home.ungrouped[1].status, RowStatus::Inactive);
        assert_eq!(home.ungrouped[1].age, "59m");
    }

    #[test]
    fn a_quiet_group_stays_folded_and_changes_replace_sessions() {
        let mut mirror = Mirror {
            sessions: vec![session("quiet", Status::Done, 1, Some(1))],
            groups: vec![group(1, "Ops")],
            usage: Vec::new(),
        };
        assert!(!mirror.home(10).groups[0].active);
        mirror.session_changed(session("quiet", Status::NeedsYou, 2, Some(1)));
        assert_eq!(mirror.sessions.len(), 1);
        assert!(mirror.home(10).groups[0].active);
    }

    #[test]
    fn background_work_shows_its_word_and_detail() {
        let mut busy = session("busy", Status::Done, 1, None);
        busy.background.agents = 1;
        busy.background.agent_title = Some("Phase 4a capture".into());
        let mirror = Mirror {
            sessions: vec![busy, session("plain", Status::Done, 2, None)],
            ..Mirror::default()
        };
        let home = mirror.home(10);
        let busy = home.ungrouped.iter().find(|row| row.id == "busy").unwrap();
        assert_eq!(busy.beside, Some(RowBeside::Background));
        assert_eq!(
            busy.beside_detail.as_deref(),
            Some("1 agent · Phase 4a capture")
        );
        let plain = home.ungrouped.iter().find(|row| row.id == "plain").unwrap();
        assert_eq!(plain.beside, None);
        assert_eq!(home.ungrouped[0].id, "busy", "rises like working");
    }

    #[test]
    fn a_reply_with_background_work_stays_in_its_group() {
        let mut busy = session("busy", Status::DoneUnread, 1, None);
        busy.background.commands = 1;
        let quiet = session("quiet", Status::DoneUnread, 2, None);
        let mirror = Mirror {
            sessions: vec![busy.clone(), quiet],
            ..Mirror::default()
        };
        let home = mirror.home(10);
        let alerts: Vec<&str> = home.alerts.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(alerts, ["quiet"]);
        let row = home.ungrouped.iter().find(|row| row.id == "busy").unwrap();
        assert_eq!(row.beside, Some(RowBeside::Background));
        busy.background.commands = 0;
        let mirror = Mirror {
            sessions: vec![busy],
            ..Mirror::default()
        };
        let alerts: Vec<String> = mirror
            .home(10)
            .alerts
            .into_iter()
            .map(|row| row.id)
            .collect();
        assert_eq!(alerts, ["busy"], "rises once the work ends");
    }

    #[test]
    fn past_conversations_are_those_outside_the_sidebar_and_offered() {
        let mut outside = session("outside", Status::Inactive, 2, None);
        outside.in_sidebar = false;
        let mut hidden = session("hidden", Status::Inactive, 3, None);
        hidden.in_sidebar = false;
        hidden.offered = false;
        let mirror = Mirror {
            sessions: vec![session("inside", Status::Done, 1, None), outside, hidden],
            groups: Vec::new(),
            usage: Vec::new(),
        };
        let past: Vec<String> = mirror.past(10).into_iter().map(|past| past.id).collect();
        assert_eq!(past, ["outside"]);
    }
}
