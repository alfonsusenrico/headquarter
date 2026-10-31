//! What the sidebar shows, derived from the host's sessions and the owner's groups.
//! Pure, so it is tested without a window. Order comes from `hq_protocol::sidebar`,
//! which `hq ls` uses too.

use std::collections::HashMap;
use std::time::Duration;

use hq_protocol::sidebar::{self, age};
use hq_protocol::{GroupColor, GroupInfo, SessionInfo};
use hq_signals::event::Agent;
use hq_signals::status::Status;

/// A status glyph; each has its own shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Glyph {
    Working,
    NeedsYou,
    IdleUnread,
    Idle,
    /// Its agent is not running; the owner resumes it (add-inactive-sessions).
    Inactive,
    /// The host is starting it with itself, until its agent is ready.
    Starting,
    /// Running in another terminal: no status is shown until it runs here.
    Elsewhere,
}

impl Glyph {
    pub fn of(session: &SessionInfo) -> Glyph {
        if session.external.is_some() {
            return Glyph::Elsewhere;
        }
        if session.starting {
            return Glyph::Starting;
        }
        if !session.live {
            return Glyph::Inactive;
        }
        Glyph::from_status(session.status)
    }

    fn from_status(status: Status) -> Glyph {
        match status {
            Status::Working => Glyph::Working,
            Status::NeedsYou => Glyph::NeedsYou,
            Status::DoneUnread => Glyph::IdleUnread,
            Status::Done => Glyph::Idle,
            Status::Inactive => Glyph::Inactive,
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
    /// The paired phone that has the session, shown with a phone mark in place of the
    /// age (refine-in-use-elsewhere D2).
    pub phone: Option<String>,
    /// "background" or "review" beside an idle session (add-background-work-status D4).
    pub beside: Option<sidebar::Beside>,
}

impl Row {
    /// Done, not read yet: the title is bold.
    pub fn bold(&self) -> bool {
        self.glyph == Glyph::IdleUnread
    }

    /// Inactive: the title and age are dimmed.
    pub fn dimmed(&self) -> bool {
        self.glyph == Glyph::Inactive
    }

    /// The status word breathes: the agent works, or is starting with the host.
    pub fn breathing(&self) -> bool {
        matches!(self.glyph, Glyph::Working | Glyph::Starting)
    }

    /// Whether the row ends with a cell for its glyph. A breathing word carries its dots
    /// instead, so it ends the row itself, right-aligned where the glyphs end (owner,
    /// 2026-10-26); "background" and "review" keep the cell for their marks.
    pub fn glyph_cell(&self) -> bool {
        !self.breathing()
    }

    /// "background" or "review", which, like "working", ends the row in place of the age
    /// (add-background-work-status D4).
    pub fn beside_word(&self) -> Option<&'static str> {
        match self.beside? {
            sidebar::Beside::Background => Some("background"),
            sidebar::Beside::Review => Some("review"),
        }
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
            Glyph::Starting => Some("starting"),
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
        /// The colour shown, and whether the owner picked it (add-group-colors).
        color: GroupColor,
        picked: Option<GroupColor>,
        /// A folded group's "2 background · 1 review", since its rows are hidden
        /// (add-background-work-status D5).
        beside_note: Option<String>,
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
            phone: session.phone.clone(),
            beside: sidebar::beside(session),
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
            color: group.shown_color(),
            picked: group.color.as_deref().and_then(GroupColor::from_name),
            beside_note: group
                .collapsed
                .then(|| beside_note(&section.sessions))
                .flatten(),
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
        color: GroupColor,
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
                color,
                ..
            } => {
                line(&mut rail);
                if *collapsed {
                    rail.push(RailItem::Chip {
                        id: *id,
                        name: name.clone(),
                        initials: initials(name),
                        total: *total,
                        waiting: *needing > 0,
                        color: *color,
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

/// How often the waiting dot's pulse steps: 12 times a second, enough for a fade to look
/// smooth. Motion is drawn on a clock rather than every display frame
/// (fix-sidebar-motion-redraw).
pub const MOTION_STEP: Duration = Duration::from_nanos(1_000_000_000 / 12);
/// How often a working row's dots step: 4 times a second (refresh-pane-and-status D1).
pub const DOTS_STEP: Duration = Duration::from_millis(250);

/// How fast the motion clock must run for what moves. Each step draws the whole window,
/// so the clock runs only as fast as the fastest motion on screen needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionRate {
    /// Only working or starting rows move: their dots need 4 steps a second.
    Dots,
    /// A collapsed group's waiting dot pulses, which needs 12 steps a second.
    Pulse,
}

impl MotionRate {
    pub fn step(self) -> Duration {
        match self {
            MotionRate::Dots => DOTS_STEP,
            MotionRate::Pulse => MOTION_STEP,
        }
    }
}

/// What the sidebar's motion needs: the waiting dot of a collapsed group waiting on the
/// owner, a working or starting row's dots, or nothing.
pub fn motion_rate(items: &[Item]) -> Option<MotionRate> {
    let mut rate = None;
    for item in items {
        match item {
            Item::GroupHeader {
                needing, collapsed, ..
            } if *collapsed && *needing > 0 => return Some(MotionRate::Pulse),
            Item::Row(row) if row.breathing() => rate = Some(MotionRate::Dots),
            _ => {}
        }
    }
    rate
}

/// Which of a working row's three dots is lit after `elapsed`: they light one after
/// another, then all rest for a step, one turn a second. `None` is the resting step.
pub fn lit_dot(elapsed: Duration) -> Option<usize> {
    let step = (elapsed.as_nanos() / DOTS_STEP.as_nanos()) % 4;
    (step < 3).then_some(step as usize)
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

/// How many of a folded group's sessions show "background" and "review", as its header
/// adds them; `None` when none do.
pub fn beside_note(sessions: &[&SessionInfo]) -> Option<String> {
    let count = |wanted| {
        sessions
            .iter()
            .filter(|session| sidebar::beside(session) == Some(wanted))
            .count()
    };
    let parts: Vec<String> = [
        (count(sidebar::Beside::Background), "background"),
        (count(sidebar::Beside::Review), "review"),
    ]
    .into_iter()
    .filter(|(count, _)| *count > 0)
    .map(|(count, word)| format!("{count} {word}"))
    .collect();
    (!parts.is_empty()).then(|| parts.join(" · "))
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

/// The session kept from the last launch, while it is still in the sidebar: one closed
/// or removed since is no longer the owner's to show.
pub fn kept_session<'a>(sessions: &'a [SessionInfo], id: &str) -> Option<&'a SessionInfo> {
    sessions
        .iter()
        .find(|session| session.id == id && session.in_sidebar)
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

/// A row's place in the sidebar: a session can be listed twice (pinned and in its group).
pub fn row_key(item: &Item) -> Option<String> {
    match item {
        Item::Row(row) => Some(format!("{}:{}", row.section.key(), row.id)),
        _ => None,
    }
}

/// `items` with every row that was in `shown` back in the order it had there, each list on
/// its own; a row new to its list keeps its computed place. Rows still carry their new
/// status, word and age (add-working-sessions-first D2).
pub fn hold(mut items: Vec<Item>, shown: &[String]) -> Vec<Item> {
    let rank: HashMap<&str, usize> = shown
        .iter()
        .enumerate()
        .map(|(place, key)| (key.as_str(), place))
        .collect();
    let mut start = 0;
    while start < items.len() {
        if !matches!(items[start], Item::Row(_)) {
            start += 1;
            continue;
        }
        let mut end = start;
        while end < items.len() && matches!(items[end], Item::Row(_)) {
            end += 1;
        }
        let place = |item: &Item| row_key(item).and_then(|key| rank.get(key.as_str()).copied());
        let slots: Vec<usize> = (start..end)
            .filter(|&at| place(&items[at]).is_some())
            .collect();
        let mut known: Vec<Item> = slots.iter().map(|&at| items[at].clone()).collect();
        known.sort_by_key(|item| place(item));
        for (slot, item) in slots.into_iter().zip(known) {
            items[slot] = item;
        }
        start = end;
    }
    items
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
            starting: false,
            start_error: None,
            offered: true,
            phone: None,
            background: Default::default(),
            review: false,
        }
    }

    #[test]
    fn held_rows_keep_their_places_and_new_ones_take_theirs() {
        let order = |items: &[Item]| -> Vec<String> {
            items
                .iter()
                .filter_map(|item| match item {
                    Item::Row(row) => {
                        Some(format!("{}:{}", row.id, row.status_word().unwrap_or("-")))
                    }
                    _ => None,
                })
                .collect()
        };
        // Shown: a finished session above an idle one.
        let before = items(
            &[
                session("finished", "/u/a", Status::Done, 1_000),
                session("cash", "/u/b", Status::Done, 9_000),
            ],
            &[],
            None,
            NOW,
        );
        let shown: Vec<String> = before.iter().filter_map(row_key).collect();
        // "cash" starts working, and a new session appears.
        let now = items(
            &[
                session("finished", "/u/a", Status::Done, 1_000),
                session("cash", "/u/b", Status::Working, 9_000),
                session("new", "/u/c", Status::Done, 0),
            ],
            &[],
            None,
            NOW,
        );
        assert_eq!(
            order(&now),
            ["cash:working", "new:-", "finished:-"],
            "computed"
        );
        let held = hold(now.clone(), &shown);
        assert_eq!(
            order(&held),
            ["finished:-", "new:-", "cash:working"],
            "held rows keep their order and show their new status; the new one keeps its place"
        );
        assert_eq!(
            hold(now.clone(), &[]),
            now,
            "nothing held: the computed order"
        );
    }

    #[test]
    fn background_and_review_end_the_row_and_fold_into_the_group() {
        let mut background = session("bg", "/u/a", Status::Done, 0);
        background.background.commands = 2;
        background.group = Some(1);
        let mut review = session("rv", "/u/b", Status::DoneUnread, 0);
        review.review = true;
        review.group = Some(1);
        let open = items(
            &[background.clone(), review.clone()],
            &[group_info(1, false)],
            None,
            NOW,
        );
        let row = &rows(&open, "bg")[0];
        assert_eq!(row.beside_word(), Some("background"));
        assert!(row.glyph_cell(), "the stacked layers end the row");
        assert_eq!(rows(&open, "rv")[0].beside_word(), Some("review"));
        assert!(
            rows(&open, "rv")[0].glyph_cell(),
            "the diamond ends the row"
        );
        let header_note = |items: &[Item]| {
            items.iter().find_map(|item| match item {
                Item::GroupHeader { beside_note, .. } => Some(beside_note.clone()),
                _ => None,
            })
        };
        assert_eq!(
            header_note(&open),
            Some(None),
            "open groups show it on the rows"
        );
        let folded = items(&[background, review], &[group_info(1, true)], None, NOW);
        assert_eq!(
            header_note(&folded),
            Some(Some("1 background · 1 review".to_owned()))
        );
    }

    #[test]
    fn a_session_a_phone_has_names_the_phone() {
        let mut on_phone = session("p", "/u/api", Status::Working, 0);
        on_phone.phone = Some("Pixel 8".into());
        let sessions = [on_phone, session("d", "/u/web", Status::Done, 0)];
        let items = items(&sessions, &[], None, NOW);
        assert_eq!(rows(&items, "p")[0].phone.as_deref(), Some("Pixel 8"));
        assert_eq!(rows(&items, "p")[0].status_word(), Some("working"), "kept");
        assert_eq!(rows(&items, "d")[0].phone, None);
    }

    #[test]
    fn closed_in_the_other_terminal_becomes_inactive() {
        let mut elsewhere = session("s", "/u/api", Status::Done, 0);
        elsewhere.live = false;
        elsewhere.external = Some(External {
            pid: Some(5),
            status: None,
            background: false,
        });
        assert_eq!(Glyph::of(&elsewhere), Glyph::Elsewhere);
        let mut closed = elsewhere.clone();
        closed.external = None;
        assert_eq!(Glyph::of(&closed), Glyph::Inactive);
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
            color: None,
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
            status: Some(Status::Working),
            background: false,
        });
        let mut not_added = session("loose", "/u/api", Status::Done, 1);
        not_added.in_sidebar = false;
        vec![titled, api_ask, web_ask, ended, outside, not_added]
    }

    fn groups(collapsed_web: bool) -> Vec<GroupInfo> {
        vec![group(1, "Backend", false), group(2, "Web", collapsed_web)]
    }

    fn group_info(id: i64, collapsed: bool) -> GroupInfo {
        GroupInfo {
            id,
            name: format!("Group {id}"),
            collapsed,
            color: None,
        }
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
        assert_eq!(outside.glyph, Glyph::Elsewhere);
        assert_eq!(
            outside.status_word(),
            None,
            "working in another terminal shows only ext and the age"
        );
        let ended = &rows(&items, "web-old")[0];
        assert_eq!(
            ended.glyph,
            Glyph::Inactive,
            "a stopped agent's session is inactive"
        );
        assert!(ended.dimmed());
        assert_eq!(ended.status_word(), None, "inactive keeps its age");
        assert_eq!(ended.path, "/u/web", "the tooltip's full folder");
        assert_eq!(items[5].target(), Some(Target::Group(1)));
        assert_eq!(items[2].target(), None, "pinned rows are not a drop target");
    }

    #[test]
    fn group_headers_carry_their_colour() {
        let mut colored = groups(false);
        colored[0].color = Some("red".into());
        let items = items(&fixture(), &colored, None, NOW);
        let headers: Vec<(i64, GroupColor, Option<GroupColor>)> = items
            .iter()
            .filter_map(|item| match item {
                Item::GroupHeader {
                    id, color, picked, ..
                } => Some((*id, *color, *picked)),
                _ => None,
            })
            .collect();
        assert!(
            headers.contains(&(colored[0].id, GroupColor::Red, Some(GroupColor::Red))),
            "a picked colour wins: {headers:?}"
        );
        assert!(
            headers.contains(&(2, GroupColor::Green, None)),
            "otherwise the colour follows the id: {headers:?}"
        );
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
            color: GroupColor::Green,
            picked: None,
            beside_note: None,
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
        assert_eq!(motion_rate(&still), None);

        let mut working = fixture();
        working[0].status = Status::Working;
        let working = items(&working, &groups(false), None, NOW);
        assert_eq!(motion_rate(&working), Some(MotionRate::Dots));

        let waiting = Item::GroupHeader {
            id: 2,
            name: "Web".into(),
            total: 1,
            needing: 1,
            collapsed: true,
            color: GroupColor::Green,
            picked: None,
            beside_note: None,
        };
        assert_eq!(
            motion_rate(std::slice::from_ref(&waiting)),
            Some(MotionRate::Pulse)
        );
        // The pulse needs the faster clock, whatever else moves.
        let both: Vec<Item> = working.iter().cloned().chain([waiting]).collect();
        assert_eq!(motion_rate(&both), Some(MotionRate::Pulse));
        let open = [Item::GroupHeader {
            id: 2,
            name: "Web".into(),
            total: 1,
            needing: 1,
            collapsed: false,
            color: GroupColor::Green,
            picked: None,
            beside_note: None,
        }];
        assert_eq!(
            motion_rate(&open),
            None,
            "an open group shows its row instead"
        );
    }

    #[test]
    fn working_dots_light_in_turn_four_steps_a_second() {
        let at = |ms| lit_dot(Duration::from_millis(ms));
        assert_eq!(at(0), Some(0));
        assert_eq!(at(249), Some(0));
        assert_eq!(at(250), Some(1));
        assert_eq!(at(500), Some(2));
        assert_eq!(at(750), None, "all rest for a step");
        assert_eq!(at(1_000), Some(0), "one turn a second");
        assert_eq!(MotionRate::Dots.step(), Duration::from_millis(250));
        assert_eq!(MotionRate::Pulse.step(), MOTION_STEP);
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
    fn sessions_starting_with_the_host_breathe_until_ready() {
        let mut waiting = session("waiting", "/u/api", Status::Inactive, 0);
        waiting.live = false;
        waiting.starting = true;
        let mut launched = waiting.clone();
        launched.id = "launched".into();
        launched.live = true;
        launched.status = Status::Done;
        let mut ready = launched.clone();
        ready.id = "ready".into();
        ready.starting = false;
        let items = items(&[waiting, launched, ready], &[], None, NOW);
        for id in ["waiting", "launched"] {
            let row = &rows(&items, id)[0];
            assert_eq!(row.glyph, Glyph::Starting, "{id}");
            assert_eq!(row.status_word(), Some("starting"));
            assert!(row.breathing() && !row.dimmed());
            assert!(!row.glyph_cell(), "the word ends the row");
        }
        assert_eq!(rows(&items, "ready")[0].glyph, Glyph::Idle);
        assert!(rows(&items, "ready")[0].glyph_cell());
        assert_eq!(motion_rate(&items), Some(MotionRate::Dots));
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
