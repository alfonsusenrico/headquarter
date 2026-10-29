//! How the sidebar arranges its sessions, shared by `hq ls`, the app and the phone so all
//! show the same thing: sessions needing the owner first, then the owner's groups, then the
//! sessions in no group. Inside each list, sessions needing the owner come first, then
//! working ones, then the rest, each by most recent activity (add-working-sessions-first).

use std::collections::HashMap;

use hq_signals::status::Status;

use crate::{GroupInfo, Origin, SessionInfo, group_labels};

pub struct Layout<'a> {
    /// Sessions needing the owner, longest waiting first; also listed in their group or
    /// the ungrouped list.
    pub needs_you: Vec<&'a SessionInfo>,
    /// The owner's groups, in the owner's order, including empty ones.
    pub groups: Vec<GroupSection<'a>>,
    /// Sidebar sessions in no group (or in a group that no longer exists), in
    /// [`band`] order, most recently active first within each.
    pub ungrouped: Vec<&'a SessionInfo>,
    /// Each sidebar folder's label: its final name, with parents added only to tell
    /// same-named folders apart, and `~` for the home folder.
    pub folders: HashMap<&'a str, String>,
}

pub struct GroupSection<'a> {
    pub group: &'a GroupInfo,
    /// The group's sessions, in [`band`] order, most recently active first within each.
    pub sessions: Vec<&'a SessionInfo>,
    pub needing: usize,
}

impl Layout<'_> {
    /// The label of a session's folder.
    pub fn folder<'s>(&'s self, session: &'s SessionInfo) -> &'s str {
        self.folders
            .get(session.cwd.as_str())
            .map_or(session.cwd.as_str(), String::as_str)
    }
}

/// Age in the sidebar's style: `0m`, `12m`, `3h`, `2d`, `5w`. Seconds are not shown,
/// so a row does not tick every few seconds (owner, 2026-10-21).
pub fn age(elapsed_ms: u64) -> String {
    let secs = elapsed_ms / 1000;
    match secs {
        0..3_600 => format!("{}m", secs / 60),
        3_600..86_400 => format!("{}h", secs / 3_600),
        86_400..1_209_600 => format!("{}d", secs / 86_400),
        _ => format!("{}w", secs / 604_800),
    }
}

/// A folder as people read it: the home folder and anything under it start with `~`.
pub fn home_path(folder: &str, home: Option<&str>) -> String {
    let Some(home) = home.map(|home| home.trim_end_matches('/')) else {
        return folder.to_owned();
    };
    if folder == home {
        "~".to_owned()
    } else if let Some(rest) = folder
        .strip_prefix(home)
        .filter(|rest| rest.starts_with('/'))
    {
        format!("~{rest}")
    } else {
        folder.to_owned()
    }
}

/// What a past conversation with no title is called (owner, 2026-10-23).
pub const UNTITLED: &str = "Untitled conversation";

/// What a session is called on screen: the owner's name when they gave one, else the
/// agent's own title on one line, else, for a past conversation, "Untitled
/// conversation" and its short id, else the generated name.
pub fn display_title(session: &SessionInfo) -> String {
    match (&session.title, untitled_id(session)) {
        (Some(title), _) if !session.owner_named => {
            title.split_whitespace().collect::<Vec<_>>().join(" ")
        }
        (_, Some(id)) => format!("{UNTITLED} · {id}"),
        _ => session.name.clone(),
    }
}

/// The short id of a past conversation its agent gave no title and the owner no name:
/// the part of its generated name (`claude-e5a1fa64`) that `hq resume` takes after the
/// agent's prefix.
pub fn untitled_id(session: &SessionInfo) -> Option<&str> {
    if session.origin != Origin::History || session.title.is_some() || session.owner_named {
        return None;
    }
    session
        .name
        .strip_prefix(session.agent.id())
        .and_then(|rest| rest.strip_prefix('-'))
        .filter(|id| !id.is_empty())
}

/// Whether the session waits for the owner here. One running in another terminal is
/// that terminal's to answer, so it never counts (ext-rows-without-status).
pub fn needs_you(session: &SessionInfo) -> bool {
    session.external.is_none() && session.status == Status::NeedsYou
}

/// What a session at its prompt shows beside its status (add-background-work-status D4):
/// "background" while work it started still runs, "review" while agy has a plan to review.
/// A session working, needing the owner, not running, or running elsewhere shows neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Beside {
    Background,
    Review,
}

pub fn beside(session: &SessionInfo) -> Option<Beside> {
    let at_prompt = matches!(session.status, Status::Done | Status::DoneUnread);
    if !at_prompt || session.external.is_some() {
        return None;
    }
    if session.review {
        Some(Beside::Review)
    } else if !session.background.is_empty() {
        Some(Beside::Background)
    } else {
        None
    }
}

/// What runs in the background, as words: "1 agent, 2 shells, 1 monitor"; `None` when
/// nothing does.
pub fn background_counts(work: &hq_signals::work::BackgroundWork) -> Option<String> {
    let mut parts = Vec::new();
    let mut add = |count: u16, one: &str, many: &str| match count {
        0 => {}
        1 => parts.push(format!("1 {one}")),
        _ => parts.push(format!("{count} {many}")),
    };
    add(work.agents, "agent", "agents");
    add(work.commands, "shell", "shells");
    add(work.monitors, "monitor", "monitors");
    (!parts.is_empty()).then(|| parts.join(", "))
}

/// The detail after the word in a session's header (add-background-work-status D5):
/// what runs and the newest agent's task, or what waits for review.
pub fn beside_detail(session: &SessionInfo) -> Option<String> {
    match beside(session)? {
        Beside::Review => Some("plan to review (/artifact)".to_owned()),
        Beside::Background => {
            let counts = background_counts(&session.background)?;
            Some(match &session.background.agent_title {
                Some(title) => format!("{counts} · {title}"),
                None => counts,
            })
        }
    }
}

/// Where a session sorts within its list: needing the owner, then working (with
/// background and review, add-background-work-status), then the rest.
/// A session running in another terminal shows no status here, so it is among the rest.
/// A working session's activity time is when its turn started, so without this it sinks
/// below sessions that finished after that (add-working-sessions-first D1).
pub fn band(session: &SessionInfo) -> u8 {
    if needs_you(session) {
        0
    } else if (session.external.is_none() && session.status == Status::Working)
        || beside(session).is_some()
    {
        1
    } else {
        2
    }
}

/// Arranges the sessions that are in the sidebar; others are left out. `home` is the
/// owner's home folder, labelled `~`.
pub fn layout<'a>(
    sessions: &'a [SessionInfo],
    groups: &'a [GroupInfo],
    home: Option<&str>,
) -> Layout<'a> {
    let mut members: Vec<&SessionInfo> = sessions
        .iter()
        .filter(|session| session.in_sidebar)
        .collect();
    members.sort_by_key(|session| {
        (
            band(session),
            std::cmp::Reverse(session.last_activity_at),
            session.id.as_str(),
        )
    });
    let mut needing: Vec<&SessionInfo> = members
        .iter()
        .copied()
        .filter(|session| needs_you(session))
        .collect();
    needing.sort_by_key(|session| (session.last_activity_at, session.id.as_str()));

    let sections: Vec<GroupSection<'_>> = groups
        .iter()
        .map(|group| {
            let sessions: Vec<&SessionInfo> = members
                .iter()
                .copied()
                .filter(|session| session.group == Some(group.id))
                .collect();
            GroupSection {
                group,
                needing: sessions.iter().filter(|session| needs_you(session)).count(),
                sessions,
            }
        })
        .collect();
    let ungrouped: Vec<&SessionInfo> = members
        .iter()
        .copied()
        .filter(|session| {
            session
                .group
                .is_none_or(|id| !groups.iter().any(|group| group.id == id))
        })
        .collect();

    let mut paths: Vec<&str> = members.iter().map(|session| session.cwd.as_str()).collect();
    paths.sort_unstable();
    paths.dedup();
    let mut folders = group_labels(&paths);
    if let Some(home) = home.map(|home| home.trim_end_matches('/'))
        && let Some(label) = folders.get_mut(home)
    {
        *label = "~".to_owned();
    }
    Layout {
        needs_you: needing,
        groups: sections,
        ungrouped,
        folders,
    }
}

#[cfg(test)]
mod tests {
    use hq_signals::event::Agent;

    use super::*;
    use crate::{External, Origin};

    #[test]
    fn a_host_that_predates_offering_counts_as_offering_everything() {
        let mut sent = serde_json::to_value(session("a", "/w", Status::Done, 1)).unwrap();
        sent.as_object_mut().unwrap().remove("offered");
        let read: SessionInfo = serde_json::from_value(sent).unwrap();
        assert!(read.offered);
    }

    fn session(id: &str, cwd: &str, status: Status, at: u64) -> SessionInfo {
        SessionInfo {
            id: id.to_owned(),
            agent: Agent::Claude,
            cwd: cwd.to_owned(),
            name: id.to_owned(),
            agent_session_id: None,
            status,
            needs_reason: None,
            created_at: 0,
            last_activity_at: at,
            ended_at: None,
            live: true,
            origin: Origin::Hosted,
            title: None,
            external: None,
            in_sidebar: true,
            group: None,
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

    fn ids(sessions: &[&SessionInfo]) -> Vec<String> {
        sessions.iter().map(|session| session.id.clone()).collect()
    }

    #[test]
    fn a_past_conversation_without_a_title_is_untitled_with_its_id() {
        let mut session = session("a", "/w", Status::Done, 0);
        session.origin = Origin::History;
        session.name = "claude-e5a1fa64".into();
        assert_eq!(untitled_id(&session), Some("e5a1fa64"));
        assert_eq!(display_title(&session), "Untitled conversation · e5a1fa64");
        session.title = Some("Fix login".into());
        assert_eq!(untitled_id(&session), None);
        assert_eq!(display_title(&session), "Fix login");
        session.title = None;
        session.name = "Smoke test".into();
        session.owner_named = true;
        assert_eq!(display_title(&session), "Smoke test");
        let mut hosted = session.clone();
        hosted.owner_named = false;
        hosted.origin = Origin::Hosted;
        hosted.name = "claude-1".into();
        assert_eq!(
            display_title(&hosted),
            "claude-1",
            "only past conversations"
        );
    }

    #[test]
    fn the_owners_name_wins_over_the_agents_title() {
        let mut session = session("a", "/w", Status::Done, 0);
        session.name = "claude-3".into();
        assert_eq!(display_title(&session), "claude-3");
        session.title = Some("Fix the\n  login flow".into());
        assert_eq!(display_title(&session), "Fix the login flow");
        session.name = "Smoke test".into();
        session.owner_named = true;
        assert_eq!(display_title(&session), "Smoke test");
    }

    #[test]
    fn needs_you_then_groups_in_order_then_ungrouped() {
        let mut outside = session("outside", "/u/web", Status::Inactive, 5);
        outside.external = Some(External {
            pid: Some(9),
            status: Some(Status::NeedsYou),
            background: false,
        });
        let mut not_added = session("not-added", "/u/api", Status::NeedsYou, 1);
        not_added.in_sidebar = false;
        let mut asking = session("api-asking", "/u/api", Status::NeedsYou, 20);
        asking.group = Some(2);
        let mut old = session("api-old", "/u/api", Status::Done, 10);
        old.group = Some(2);
        let mut lost = session("lost", "/u/api", Status::Done, 40);
        lost.group = Some(99);
        let sessions = [
            old,
            asking,
            session("web-new", "/u/web", Status::Working, 50),
            session("api-new", "/u/api", Status::Done, 30),
            outside,
            not_added,
            lost,
        ];
        let groups = [group(2, "Backend"), group(1, "Empty")];
        let layout = layout(&sessions, &groups, None);
        assert_eq!(
            ids(&layout.needs_you),
            ["api-asking"],
            "one asking in another terminal is not listed"
        );
        let names: Vec<&str> = layout
            .groups
            .iter()
            .map(|section| section.group.name.as_str())
            .collect();
        assert_eq!(names, ["Backend", "Empty"], "the owner's order");
        let backend = &layout.groups[0];
        assert_eq!(
            ids(&backend.sessions),
            ["api-asking", "api-old"],
            "sessions needing the owner stay in their group too"
        );
        assert_eq!((backend.sessions.len(), backend.needing), (2, 1));
        assert!(layout.groups[1].sessions.is_empty());
        assert_eq!(
            ids(&layout.ungrouped),
            ["web-new", "lost", "api-new", "outside"],
            "working first, then most recent; a vanished group's sessions are ungrouped"
        );
    }

    #[test]
    fn background_and_review_sort_with_working() {
        let mut background = session("bg", "/u", Status::Done, 5);
        background.background.commands = 2;
        let mut review = session("review", "/u", Status::DoneUnread, 4);
        review.review = true;
        let mut busy = session("busy", "/u", Status::Working, 3);
        busy.background.agents = 1;
        let mut asking = session("asking", "/u", Status::NeedsYou, 2);
        asking.background.commands = 1;
        let sessions = [
            session("newest-idle", "/u", Status::Done, 9),
            background,
            review,
            busy,
            asking,
        ];
        assert_eq!(beside(&sessions[1]), Some(Beside::Background));
        assert_eq!(beside(&sessions[2]), Some(Beside::Review));
        assert_eq!(beside(&sessions[3]), None, "working shows working");
        assert_eq!(beside(&sessions[4]), None, "needs you wins");
        let layout = layout(&sessions, &[], None);
        assert_eq!(
            ids(&layout.ungrouped),
            ["asking", "bg", "review", "busy", "newest-idle"]
        );
    }

    #[test]
    fn background_reads_as_counts_and_the_agents_task() {
        let mut at_prompt = session("s", "/u", Status::Done, 0);
        assert_eq!(beside_detail(&at_prompt), None);
        at_prompt.background.agents = 1;
        at_prompt.background.commands = 2;
        assert_eq!(
            beside_detail(&at_prompt).as_deref(),
            Some("1 agent, 2 shells")
        );
        at_prompt.background.agent_title = Some("Phase 4a capture".into());
        at_prompt.background.monitors = 1;
        assert_eq!(
            beside_detail(&at_prompt).as_deref(),
            Some("1 agent, 2 shells, 1 monitor · Phase 4a capture")
        );
        at_prompt.review = true;
        assert_eq!(
            beside_detail(&at_prompt).as_deref(),
            Some("plan to review (/artifact)")
        );
    }

    #[test]
    fn needing_you_then_working_then_the_rest_in_each_list() {
        let in_group = |mut session: SessionInfo| {
            session.group = Some(1);
            session
        };
        let mut elsewhere = session("ext-working", "/u", Status::Working, 99);
        elsewhere.external = Some(External {
            pid: Some(3),
            status: Some(Status::Working),
            background: false,
        });
        let sessions = [
            // Working for 20 minutes: its activity is when the turn started.
            in_group(session("cash-flow", "/u", Status::Working, 10)),
            // Finished 17 minutes ago: newer activity, but not working.
            in_group(session("headquarter", "/u", Status::DoneUnread, 13)),
            in_group(session("asking", "/u", Status::NeedsYou, 5)),
            in_group(session("older-working", "/u", Status::Working, 8)),
            in_group(session("idle", "/u", Status::Done, 30)),
            in_group(elsewhere),
        ];
        let groups = [group(1, "Personal")];
        let layout = layout(&sessions, &groups, None);
        assert_eq!(
            ids(&layout.groups[0].sessions),
            [
                "asking",
                "cash-flow",
                "older-working",
                "ext-working",
                "idle",
                "headquarter"
            ],
            "needs you, then working by recent activity, then the rest; ext has no status"
        );
        assert_eq!(
            ids(&layout.needs_you),
            ["asking"],
            "the section is unchanged"
        );
    }

    #[test]
    fn folder_labels_tell_apart_and_shorten_home() {
        let sessions = [
            session("a", "/u/one/api", Status::Done, 1),
            session("b", "/u/two/api", Status::Done, 2),
            session("c", "/home/me", Status::Done, 3),
        ];
        let layout = layout(&sessions, &[], Some("/home/me/"));
        assert_eq!(layout.folder(&sessions[0]), "one/api");
        assert_eq!(layout.folder(&sessions[1]), "two/api");
        assert_eq!(layout.folder(&sessions[2]), "~");
    }

    #[test]
    fn home_paths_start_with_a_tilde() {
        assert_eq!(home_path("/home/me", Some("/home/me")), "~");
        assert_eq!(
            home_path("/home/me/work/api", Some("/home/me/")),
            "~/work/api"
        );
        assert_eq!(home_path("/home/meadow", Some("/home/me")), "/home/meadow");
        assert_eq!(home_path("/srv/app", None), "/srv/app");
    }

    #[test]
    fn ages() {
        assert_eq!(age(1_000), "0m");
        assert_eq!(age(59_999), "0m");
        assert_eq!(age(60_000), "1m");
        assert_eq!(age(12 * 60_000), "12m");
        assert_eq!(age(3 * 3_600_000), "3h");
        assert_eq!(age(2 * 86_400_000), "2d");
        assert_eq!(age(5 * 604_800_000), "5w");
    }

    #[test]
    fn empty_when_nothing_is_added() {
        let mut only = session("a", "/w", Status::Done, 1);
        only.in_sidebar = false;
        let sessions = [only];
        let layout = layout(&sessions, &[], None);
        assert!(layout.needs_you.is_empty() && layout.ungrouped.is_empty());
    }
}
