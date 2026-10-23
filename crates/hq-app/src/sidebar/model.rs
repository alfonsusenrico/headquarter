//! What the sidebar shows, derived from the host's sessions and the owner's groups.
//! Pure, so it is tested without a window. Order comes from `hq_protocol::sidebar`,
//! which `hq ls` uses too.

use std::time::Duration;

use hq_protocol::sidebar::{self, age};
use hq_protocol::{GroupInfo, SessionInfo};
use hq_signals::event::Agent;
use hq_signals::status::Status;

/// A status glyph; each has its own shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Working,
    NeedsYou,
    IdleUnread,
    Idle,
    /// Running in another terminal with no status reported.
    Unknown,
}

impl Glyph {
    pub fn of(session: &SessionInfo) -> Glyph {
        if let Some(external) = &session.external {
            return external.status.map_or(Glyph::Unknown, Glyph::from_status);
        }
        // A session whose agent is not running is idle: opening it resumes the agent.
        if !session.live {
            return Glyph::Idle;
        }
        Glyph::from_status(session.status)
    }

    fn from_status(status: Status) -> Glyph {
        match status {
            Status::Working => Glyph::Working,
            Status::NeedsYou => Glyph::NeedsYou,
            Status::DoneUnread => Glyph::IdleUnread,
            Status::Done => Glyph::Idle,
            Status::Inactive => Glyph::Idle,
        }
    }
}

/// Where a row is listed. A session needing the owner is listed twice: pinned, and
/// in its group or the ungrouped list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Pinned,
    Group(i64),
    Ungrouped,
}

impl Section {
    /// A stable piece of element ids, so a session's two rows stay apart.
    pub fn key(self) -> String {
        match self {
            Section::Pinned => "pinned".to_owned(),
            Section::Group(id) => format!("group{id}"),
            Section::Ungrouped => "ungrouped".to_owned(),
        }
    }
}

/// Where a dragged session can be dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    Group(i64),
    Ungrouped,
    NewGroup,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub agent: Agent,
    pub title: String,
    /// The folder's label: its final name, with parents only to tell folders apart.
    pub folder: String,
    /// The full folder, the home folder as `~`; the row's tooltip.
    pub path: String,
    pub section: Section,
    /// Running in another terminal: shown with "ext" and observed only.
    pub external: bool,
    pub age: String,
    pub glyph: Glyph,
}

impl Row {
    /// Done, not read yet: the title is bold.
    pub fn bold(&self) -> bool {
        self.glyph == Glyph::IdleUnread
    }

    /// In the pinned "Needs you" section.
    pub fn pinned(&self) -> bool {
        self.section == Section::Pinned
    }

    /// States that need a word as well as a glyph, shown in place of the age so the
    /// owner does not have to learn the glyphs first. Pinned rows keep their age: the
    /// section's heading already names the state.
    pub fn status_word(&self) -> Option<&'static str> {
        if self.pinned() {
            return None;
        }
        match self.glyph {
            Glyph::Working => Some("working"),
            Glyph::NeedsYou => Some("needs you"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    NeedsYouHeader {
        count: usize,
    },
    GroupHeader {
        id: i64,
        name: String,
        total: usize,
        needing: usize,
        collapsed: bool,
    },
    /// The line that adds a group; also a drop target.
    NewGroup,
    /// Labels the sessions in no group once any group exists.
    UngroupedHeader {
        total: usize,
    },
    Row(Row),
    /// Sets the pinned "Needs you" section apart from the rest.
    Divider,
}

impl Item {
    /// The drop target this item belongs to, if any.
    pub fn target(&self) -> Option<Target> {
        match self {
            Item::GroupHeader { id, .. } => Some(Target::Group(*id)),
            Item::NewGroup => Some(Target::NewGroup),
            Item::UngroupedHeader { .. } => Some(Target::Ungrouped),
            Item::Row(row) => match row.section {
                Section::Pinned => None,
                Section::Group(id) => Some(Target::Group(id)),
                Section::Ungrouped => Some(Target::Ungrouped),
            },
            Item::NeedsYouHeader { .. } | Item::Divider => None,
        }
    }
}

/// The sidebar's items from top to bottom. Rows of collapsed groups are left out;
/// their sessions needing the owner still show in the "Needs you" section.
pub fn items(
    sessions: &[SessionInfo],
    groups: &[GroupInfo],
    home: Option<&str>,
    now_ms: u64,
) -> Vec<Item> {
    let layout = sidebar::layout(sessions, groups, home);
    let row = |session: &SessionInfo, section: Section| {
        Item::Row(Row {
            id: session.id.clone(),
            agent: session.agent,
            title: sidebar::display_title(session),
            folder: layout.folder(session).to_owned(),
            path: sidebar::home_path(&session.cwd, home),
            section,
            external: session.external.is_some(),
            age: age(now_ms.saturating_sub(session.last_activity_at)),
            glyph: Glyph::of(session),
        })
    };
    let mut items = Vec::new();
    let anything_else = !layout.groups.is_empty() || !layout.ungrouped.is_empty();
    // Always first, so a group can be made from anywhere in a long list (D3).
    if anything_else {
        items.push(Item::NewGroup);
    }
    if !layout.needs_you.is_empty() {
        items.push(Item::NeedsYouHeader {
            count: layout.needs_you.len(),
        });
        items.extend(
            layout
                .needs_you
                .iter()
                .map(|session| row(session, Section::Pinned)),
        );
        if anything_else {
            items.push(Item::Divider);
        }
    }
    for section in &layout.groups {
        let group = section.group;
        items.push(Item::GroupHeader {
            id: group.id,
            name: group.name.clone(),
            total: section.sessions.len(),
            needing: section.needing,
            collapsed: group.collapsed,
        });
        if !group.collapsed {
            items.extend(
                section
                    .sessions
                    .iter()
                    .map(|session| row(session, Section::Group(group.id))),
            );
        }
    }
    // Labelled whenever something is ungrouped, groups or not (label-ungrouped-sessions).
    if !layout.ungrouped.is_empty() {
        items.push(Item::UngroupedHeader {
            total: layout.ungrouped.len(),
        });
    }
    items.extend(
        layout
            .ungrouped
            .iter()
            .map(|session| row(session, Section::Ungrouped)),
    );
    items
}

/// What the icon rail shows, top to bottom (add-sidebar-rail D3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RailItem {
    Session(Row),
    /// A collapsed group: its initials and size; `waiting` when a session in it needs
    /// the owner.
    Chip {
        id: i64,
        name: String,
        initials: String,
        total: usize,
        waiting: bool,
    },
    /// Separates "Needs you", the groups and the ungrouped sessions.
    Line,
}

/// The rail's items from the sidebar's: rows become marks, collapsed groups chips,
/// and sections are set apart by lines; headings are left out.
pub fn rail_items(items: &[Item]) -> Vec<RailItem> {
    let mut rail = Vec::new();
    let line = |rail: &mut Vec<RailItem>| {
        if !matches!(rail.last(), None | Some(RailItem::Line)) {
            rail.push(RailItem::Line);
        }
    };
    for item in items {
        match item {
            Item::Row(row) => rail.push(RailItem::Session(row.clone())),
            Item::GroupHeader {
                id,
                name,
                total,
                needing,
                collapsed,
            } => {
                line(&mut rail);
                if *collapsed {
                    rail.push(RailItem::Chip {
                        id: *id,
                        name: name.clone(),
                        initials: initials(name),
                        total: *total,
                        waiting: *needing > 0,
                    });
                }
            }
            Item::Divider | Item::UngroupedHeader { .. } => line(&mut rail),
            Item::NeedsYouHeader { .. } | Item::NewGroup => {}
        }
    }
    if matches!(rail.last(), Some(RailItem::Line)) {
        rail.pop();
    }
    rail
}

/// A group's initials for its rail chip: the first letters of its first two words, or
/// the first two letters of a single word.
pub fn initials(name: &str) -> String {
    let words: Vec<&str> = name.split_whitespace().collect();
    let letters: String = match words.as_slice() {
        [] => String::new(),
        [only] => only.chars().take(2).collect(),
        [first, second, ..] => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect(),
    };
    letters.to_uppercase()
}

/// How often sidebar motion steps: 12 times a second, enough for a fade to look smooth.
/// Motion is drawn on this clock rather than every display frame
/// (fix-sidebar-motion-redraw).
pub const MOTION_STEP: Duration = Duration::from_nanos(1_000_000_000 / 12);

/// Whether anything in the sidebar moves: a working row's "working" word, or the dot of
/// a collapsed group waiting on the owner.
pub fn in_motion(items: &[Item]) -> bool {
    items.iter().any(|item| match item {
        Item::Row(row) => row.glyph == Glyph::Working,
        Item::GroupHeader {
            needing, collapsed, ..
        } => *collapsed && *needing > 0,
        _ => false,
    })
}

/// Where a motion of `period` is after `elapsed`, from 0.0 to 1.0, moved on whole
/// steps of [`MOTION_STEP`] so every frame drawn shows a new position.
pub fn motion_phase(elapsed: Duration, period: Duration) -> f32 {
    let step = MOTION_STEP.as_nanos();
    // Whole steps per period, so a turn always ends back at 0.0.
    let per_period = (period.as_nanos() as f64 / step as f64).round().max(1.0) as u128;
    let steps = elapsed.as_nanos() / step;
    (steps % per_period) as f32 / per_period as f32
}

/// A group's size as its header says it.
pub fn session_count(count: usize) -> String {
    match count {
        1 => "1 session".to_owned(),
        _ => format!("{count} sessions"),
    }
}

/// The tooltip of a collapsed group holding sessions that need the owner.
pub fn needing_note(count: usize) -> String {
    match count {
        1 => "1 needs you".to_owned(),
        _ => format!("{count} need you"),
    }
}

/// The sidebar sessions that were running when Headquarter stopped without ending
/// them, offered for restarting together (add-session-restore D4).
pub fn interrupted(sessions: &[SessionInfo]) -> Vec<&SessionInfo> {
    sessions
        .iter()
        .filter(|session| session.in_sidebar && session.interrupted && !session.live)
        .collect()
}

/// The session kept from the last launch, while it is still in the sidebar: one closed
/// or removed since is no longer the owner's to show.
pub fn kept_session<'a>(sessions: &'a [SessionInfo], id: &str) -> Option<&'a SessionInfo> {
    sessions
        .iter()
        .find(|session| session.id == id && session.in_sidebar)
}

/// The first line of the restart note; with nothing left to offer, only failures remain.
pub fn interrupted_headline(count: usize) -> String {
    match count {
        0 => "Some sessions could not restart".to_owned(),
        1 => "1 session was running when Headquarter stopped".to_owned(),
        _ => format!("{count} sessions were running when Headquarter stopped"),
    }
}

/// The group a session moves to with the keyboard alternative to dragging: one step
/// up or down through the groups, where the ungrouped list counts as the last place.
pub fn neighbour_group(groups: &[GroupInfo], current: Option<i64>, step: isize) -> Option<i64> {
    let places: Vec<Option<i64>> = groups
        .iter()
        .map(|group| Some(group.id))
        .chain(std::iter::once(None))
        .collect();
    let here = places
        .iter()
        .position(|place| *place == current)
        .unwrap_or(places.len() - 1);
    let next = here.saturating_add_signed(step).min(places.len() - 1);
    places[next]
}

/// Ids of the rows on screen, top to bottom, for arrow keys. A session shown twice
/// (pinned and in its folder) is visited once, at its first place.
pub fn row_ids(items: &[Item]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    items
        .iter()
        .filter_map(|item| match item {
            Item::Row(row) => Some(row.id.clone()),
            _ => None,
        })
        .filter(|id| seen.insert(id.clone()))
        .collect()
}

/// The row an arrow key moves to: one step from the current row, stopping at the
/// ends; the first row when nothing is selected.
pub fn next_selection(ids: &[String], current: Option<&str>, step: isize) -> Option<String> {
    if ids.is_empty() {
        return None;
    }
    let index = match current.and_then(|id| ids.iter().position(|known| known == id)) {
        Some(index) => index.saturating_add_signed(step).min(ids.len() - 1),
        None => 0,
    };
    Some(ids[index].clone())
}

/// The session the "next needing you" key shows: the one after `current` in the
/// "Needs you" order, wrapping around; `None` when nothing needs the owner.
pub fn next_needing(sessions: &[SessionInfo], current: Option<&str>) -> Option<String> {
    let order: Vec<&str> = sidebar::layout(sessions, &[], None)
        .needs_you
        .iter()
        .map(|session| session.id.as_str())
        .collect();
    let next = match current.and_then(|id| order.iter().position(|known| *known == id)) {
        Some(index) => order.get(index + 1).or(order.first()),
        None => order.first(),
    };
    next.map(|id| (*id).to_owned())
}

#[cfg(test)]
mod tests {
    use hq_protocol::{External, Origin};

    use super::*;

    const NOW: u64 = 10 * 86_400_000;

    fn session(id: &str, cwd: &str, status: Status, idle_ms: u64) -> SessionInfo {
        SessionInfo {
            id: id.to_owned(),
            agent: Agent::Claude,
            cwd: cwd.to_owned(),
            name: id.to_owned(),
            agent_session_id: None,
            status,
            needs_reason: None,
            created_at: 0,
            last_activity_at: NOW - idle_ms,
            ended_at: None,
            live: true,
            origin: Origin::Hosted,
            title: None,
            external: None,
            in_sidebar: true,
            group: None,
            owner_named: false,
            interrupted: false,
            offered: true,
        }
    }

    #[test]
    fn a_kept_session_is_shown_only_while_in_the_sidebar() {
        let mut closed = session("closed", "/w", Status::Inactive, 0);
        closed.in_sidebar = false;
        let sessions = vec![session("kept", "/w", Status::Done, 0), closed];
        assert_eq!(
            kept_session(&sessions, "kept").map(|session| session.id.as_str()),
            Some("kept")
        );
        assert!(kept_session(&sessions, "closed").is_none());
        assert!(kept_session(&sessions, "gone").is_none());
    }

    fn group(id: i64, name: &str, collapsed: bool) -> GroupInfo {
        GroupInfo {
            id,
            name: name.to_owned(),
            collapsed,
        }
    }

    /// Groups 1 "Backend" (api sessions) and 2 "Web" (web-ask only); web-old and web-ext
    /// ungrouped.
    fn fixture() -> Vec<SessionInfo> {
        let mut titled = session("api-done", "/u/api", Status::DoneUnread, 3_600_000);
        titled.title = Some("Fix the\nlogin flow".into());
        titled.group = Some(1);
        let mut api_ask = session("api-ask", "/u/api", Status::NeedsYou, 120_000);
        api_ask.group = Some(1);
        let mut web_ask = session("web-ask", "/u/web", Status::NeedsYou, 600_000);
        web_ask.group = Some(2);
        let mut ended = session("web-old", "/u/web", Status::Inactive, 2 * 86_400_000);
        ended.live = false;
        let mut outside = session("web-ext", "/u/web", Status::Inactive, 60_000);
        outside.live = false;
        outside.external = Some(External {
            pid: Some(4),
            status: None,
        });
        let mut not_added = session("loose", "/u/api", Status::Done, 1);
        not_added.in_sidebar = false;
        vec![titled, api_ask, web_ask, ended, outside, not_added]
    }

    fn groups(collapsed_web: bool) -> Vec<GroupInfo> {
        vec![group(1, "Backend", false), group(2, "Web", collapsed_web)]
    }

    fn rows(items: &[Item], id: &str) -> Vec<Row> {
        items
            .iter()
            .filter_map(|item| match item {
                Item::Row(row) if row.id == id => Some(row.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn needs_you_then_groups_then_ungrouped() {
        let items = items(&fixture(), &groups(false), None, NOW);
        let shape: Vec<String> = items
            .iter()
            .map(|item| match item {
                Item::NeedsYouHeader { count } => format!("needs {count}"),
                Item::GroupHeader {
                    name,
                    total,
                    needing,
                    ..
                } => format!("{name} {total}/{needing}"),
                Item::NewGroup => "new".to_owned(),
                Item::UngroupedHeader { total } => format!("ungrouped {total}"),
                Item::Row(row) => format!("{}@{}", row.id, row.section.key()),
                Item::Divider => "---".to_owned(),
            })
            .collect();
        assert_eq!(
            shape,
            [
                "new",
                "needs 2",
                "web-ask@pinned",
                "api-ask@pinned",
                "---",
                "Backend 2/1",
                "api-ask@group1",
                "api-done@group1",
                "Web 1/1",
                "web-ask@group2",
                "ungrouped 2",
                "web-ext@ungrouped",
                "web-old@ungrouped",
            ]
        );
        assert_eq!(
            row_ids(&items),
            ["web-ask", "api-ask", "api-done", "web-ext", "web-old"],
            "each session once, at its first place"
        );
        let asking = &rows(&items, "web-ask");
        assert_eq!(
            asking[0].status_word(),
            None,
            "the pinned row keeps its age"
        );
        assert_eq!(asking[1].status_word(), Some("needs you"));
        assert_eq!(asking[0].folder, "web");

        let done = &rows(&items, "api-done")[0];
        assert_eq!(done.status_word(), None, "done keeps its age");
        assert_eq!(done.title, "Fix the login flow");
        assert_eq!(done.age, "1h");
        assert!(done.bold());
        let outside = &rows(&items, "web-ext")[0];
        assert!(outside.external);
        assert_eq!(outside.glyph, Glyph::Unknown);
        let ended = &rows(&items, "web-old")[0];
        assert_eq!(
            ended.glyph,
            Glyph::Idle,
            "a stopped agent's session is idle"
        );
        assert_eq!(ended.path, "/u/web", "the tooltip's full folder");
        assert_eq!(items[5].target(), Some(Target::Group(1)));
        assert_eq!(items[2].target(), None, "pinned rows are not a drop target");
    }

    #[test]
    fn collapsed_groups_hide_rows_but_not_needs_you() {
        let items = items(&fixture(), &groups(true), None, NOW);
        assert!(rows(&items, "web-ask").len() == 1, "only the pinned copy");
        assert!(items.contains(&Item::GroupHeader {
            id: 2,
            name: "Web".into(),
            total: 1,
            needing: 1,
            collapsed: true,
        }));
    }

    #[test]
    fn without_groups_the_sessions_are_still_labelled_ungrouped() {
        let mut sessions = fixture();
        for session in &mut sessions {
            session.group = None;
        }
        let items = items(&sessions, &[], None, NOW);
        assert_eq!(items[0], Item::NewGroup);
        let heading = items
            .iter()
            .position(|item| matches!(item, Item::UngroupedHeader { .. }))
            .expect("an Ungrouped heading");
        assert_eq!(items[heading - 1], Item::Divider, "below Needs you");
        let rows = items[heading + 1..]
            .iter()
            .filter(|item| matches!(item, Item::Row(_)))
            .count();
        assert_eq!(items[heading], Item::UngroupedHeader { total: rows });
        assert_eq!(rows, items.len() - heading - 1, "only rows follow it");
    }

    #[test]
    fn ungrouped_is_hidden_when_empty() {
        let mut sessions = fixture();
        for session in &mut sessions {
            session.group.get_or_insert(1);
        }
        let items = items(&sessions, &groups(false), Some("/u"), NOW);
        assert!(
            !items
                .iter()
                .any(|item| matches!(item, Item::UngroupedHeader { .. }))
        );
        assert_eq!(items[0], Item::NewGroup);
        let row = items
            .iter()
            .find_map(|item| match item {
                Item::Row(row) => Some(row.clone()),
                _ => None,
            })
            .unwrap();
        assert!(row.path.starts_with("~/"), "{}", row.path);
    }

    #[test]
    fn rail_lists_marks_chips_and_lines() {
        let items = items(&fixture(), &groups(true), None, NOW);
        let shape: Vec<String> = rail_items(&items)
            .iter()
            .map(|item| match item {
                RailItem::Session(row) => row.id.clone(),
                RailItem::Chip {
                    initials,
                    total,
                    waiting,
                    ..
                } => format!("[{initials} {total}{}]", if *waiting { " !" } else { "" }),
                RailItem::Line => "---".to_owned(),
            })
            .collect();
        assert_eq!(
            shape,
            [
                "web-ask", "api-ask", "---", "api-ask", "api-done", "---", "[WE 1 !]", "---",
                "web-ext", "web-old",
            ]
        );
    }

    #[test]
    fn group_initials() {
        assert_eq!(initials("Docs"), "DO");
        assert_eq!(initials("landing page copy"), "LP");
        assert_eq!(initials("  "), "");
        assert_eq!(initials("é"), "É");
    }

    #[test]
    fn header_words() {
        assert_eq!(session_count(1), "1 session");
        assert_eq!(session_count(0), "0 sessions");
        assert_eq!(session_count(3), "3 sessions");
        assert_eq!(needing_note(1), "1 needs you");
        assert_eq!(needing_note(2), "2 need you");
    }

    #[test]
    fn motion_comes_from_working_rows_and_waiting_collapsed_groups_only() {
        let still = items(&fixture(), &groups(false), None, NOW);
        assert!(!in_motion(&still));

        let mut working = fixture();
        working[0].status = Status::Working;
        assert!(in_motion(&items(&working, &groups(false), None, NOW)));

        let waiting = [Item::GroupHeader {
            id: 2,
            name: "Web".into(),
            total: 1,
            needing: 1,
            collapsed: true,
        }];
        assert!(in_motion(&waiting));
        let open = [Item::GroupHeader {
            id: 2,
            name: "Web".into(),
            total: 1,
            needing: 1,
            collapsed: false,
        }];
        assert!(!in_motion(&open), "an open group shows its row instead");
    }

    #[test]
    fn motion_moves_in_twelve_steps_a_second() {
        let second = Duration::from_secs(1);
        assert_eq!(motion_phase(Duration::ZERO, second), 0.0);
        assert_eq!(motion_phase(Duration::from_millis(80), second), 0.0);
        let one_step = motion_phase(Duration::from_millis(90), second);
        assert!((one_step - 1.0 / 12.0).abs() < 1e-4, "{one_step}");
        let half = motion_phase(Duration::from_millis(500), second);
        assert!((half - 0.5).abs() < 1e-4, "{half}");
        assert!(motion_phase(Duration::from_millis(1_010), second) < 1e-4);
        let slow = motion_phase(Duration::from_millis(1_010), Duration::from_secs(2));
        assert!((slow - 0.5).abs() < 1e-4, "{slow}");
    }

    #[test]
    fn the_restart_note_offers_only_stopped_sidebar_sessions_marked_interrupted() {
        let mut stopped = session("stopped", "/u/api", Status::Inactive, 0);
        stopped.live = false;
        stopped.interrupted = true;
        let mut restarted = stopped.clone();
        restarted.id = "restarted".into();
        restarted.live = true;
        let mut removed = stopped.clone();
        removed.id = "removed".into();
        removed.in_sidebar = false;
        let mut ended = stopped.clone();
        ended.id = "ended".into();
        ended.interrupted = false;
        let sessions = [stopped, restarted, removed, ended];
        let offered: Vec<&str> = interrupted(&sessions)
            .iter()
            .map(|session| session.id.as_str())
            .collect();
        assert_eq!(offered, ["stopped"]);

        assert_eq!(
            interrupted_headline(1),
            "1 session was running when Headquarter stopped"
        );
        assert_eq!(
            interrupted_headline(3),
            "3 sessions were running when Headquarter stopped"
        );
        assert_eq!(interrupted_headline(0), "Some sessions could not restart");
    }

    #[test]
    fn empty_sidebar_has_no_items() {
        let mut only = fixture();
        for session in &mut only {
            session.in_sidebar = false;
        }
        assert!(items(&only, &[], None, NOW).is_empty());
    }

    #[test]
    fn keyboard_moves_through_groups_then_ungrouped() {
        let groups = groups(false);
        assert_eq!(neighbour_group(&groups, None, -1), Some(2));
        assert_eq!(neighbour_group(&groups, Some(2), -1), Some(1));
        assert_eq!(
            neighbour_group(&groups, Some(1), -1),
            Some(1),
            "stops at the top"
        );
        assert_eq!(neighbour_group(&groups, Some(2), 1), None);
        assert_eq!(
            neighbour_group(&groups, None, 1),
            None,
            "stops at the bottom"
        );
        assert_eq!(neighbour_group(&[], None, -1), None);
    }

    #[test]
    fn arrow_keys_move_one_row_and_stop_at_the_ends() {
        let ids: Vec<String> = ["a", "b", "c"].map(String::from).to_vec();
        assert_eq!(next_selection(&ids, None, 1).as_deref(), Some("a"));
        assert_eq!(next_selection(&ids, Some("a"), 1).as_deref(), Some("b"));
        assert_eq!(next_selection(&ids, Some("c"), 1).as_deref(), Some("c"));
        assert_eq!(next_selection(&ids, Some("b"), -1).as_deref(), Some("a"));
        assert_eq!(next_selection(&ids, Some("gone"), 1).as_deref(), Some("a"));
        assert_eq!(next_selection(&[], Some("a"), 1), None);
    }

    #[test]
    fn jump_cycles_through_sessions_needing_you() {
        let sessions = fixture();
        assert_eq!(next_needing(&sessions, None).as_deref(), Some("web-ask"));
        assert_eq!(
            next_needing(&sessions, Some("web-ask")).as_deref(),
            Some("api-ask")
        );
        assert_eq!(
            next_needing(&sessions, Some("api-ask")).as_deref(),
            Some("web-ask"),
            "wraps around"
        );
        assert_eq!(
            next_needing(&sessions, Some("api-done")).as_deref(),
            Some("web-ask")
        );
        let calm: Vec<SessionInfo> = sessions
            .into_iter()
            .filter(|session| session.status != Status::NeedsYou)
            .collect();
        assert_eq!(next_needing(&calm, None), None);
    }
}
