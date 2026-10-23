//! Text rendering of the sidebar, in the owner's groups, and of every session, by
//! working folder.

use hq_protocol::sidebar::{self, age, needs_you};
use hq_protocol::usage;
use hq_protocol::{AgentUsage, GroupInfo, SessionInfo, group_labels};
use hq_signals::event::NeedsReason;
use hq_signals::status::Status;

const TITLE_WIDTH: usize = 40;

/// The sidebar as `hq ls` shows it: sessions needing the owner first, then the owner's
/// groups, then the sessions in no group.
pub fn sidebar_list(sessions: &[SessionInfo], groups: &[GroupInfo], now_ms: u64) -> String {
    let home = hq_signals::paths::home().map(|home| home.to_string_lossy().into_owned());
    let layout = sidebar::layout(sessions, groups, home.as_deref());
    if layout.ungrouped.is_empty() && layout.groups.is_empty() {
        return "The sidebar is empty. `hq ls --all` lists every session; \
                `hq add <session>` puts one in the sidebar.\n"
            .to_owned();
    }
    let mut out = String::new();
    if !layout.needs_you.is_empty() {
        out.push_str(&format!("Needs you  {}\n", layout.needs_you.len()));
        for session in &layout.needs_you {
            out.push_str(&row(session, now_ms, "  ", Some(layout.folder(session))));
        }
    }
    for section in &layout.groups {
        let collapsed = if section.group.collapsed {
            "  (collapsed)"
        } else {
            ""
        };
        out.push_str(&format!(
            "{}  {}{collapsed}\n",
            section.group.name,
            tally(section.sessions.len(), section.needing)
        ));
        for session in &section.sessions {
            out.push_str(&row(session, now_ms, "  ", Some(layout.folder(session))));
        }
    }
    if !layout.ungrouped.is_empty() {
        if !layout.groups.is_empty() {
            let needing = layout
                .ungrouped
                .iter()
                .filter(|session| needs_you(session))
                .count();
            out.push_str(&format!(
                "Ungrouped  {}\n",
                tally(layout.ungrouped.len(), needing)
            ));
        }
        for session in &layout.ungrouped {
            out.push_str(&row(session, now_ms, "  ", Some(layout.folder(session))));
        }
    }
    out
}

/// Every session the host knows (`hq ls --all`) by working folder; `*` marks those in
/// the sidebar, and a sidebar session in a group names it.
pub fn catalog_list(sessions: &[SessionInfo], groups: &[GroupInfo], now_ms: u64) -> String {
    if sessions.is_empty() {
        return "No sessions yet. Start one with `hq new claude` (or codex, agy).\n".to_owned();
    }
    let mut folders: Vec<&str> = sessions
        .iter()
        .map(|session| session.cwd.as_str())
        .collect();
    folders.sort_unstable();
    folders.dedup();
    let labels = group_labels(&folders);
    let home = hq_signals::paths::home().map(|home| home.to_string_lossy().into_owned());

    let mut out = String::new();
    for folder in folders {
        let members: Vec<&SessionInfo> = sessions
            .iter()
            .filter(|session| session.cwd == folder)
            .collect();
        let needing = members.iter().filter(|session| needs_you(session)).count();
        out.push_str(&format!(
            "{}  {}  {}\n",
            labels[folder],
            tally(members.len(), needing),
            sidebar::home_path(folder, home.as_deref())
        ));
        for session in members {
            let mark = if session.in_sidebar { "* " } else { "  " };
            let group = session
                .group
                .and_then(|id| groups.iter().find(|group| group.id == id))
                .map(|group| format!("[{}]", group.name));
            out.push_str(&row(session, now_ms, mark, group.as_deref()));
        }
    }
    out
}

/// `hq usage`: each agent's windows with share used, reset time and reading age, or
/// why its usage cannot be read.
pub fn usage_list<Tz: chrono::TimeZone>(agents: &[AgentUsage], now: &chrono::DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    if agents.is_empty() {
        return "No agents are enabled.\n".to_owned();
    }
    let now_secs = u64::try_from(now.timestamp()).unwrap_or(0);
    let mut out = String::new();
    for agent in agents {
        let name = agent.agent.to_string();
        let Some(read_at) = agent.read_at else {
            let reason = agent.problem.as_deref().unwrap_or("not read yet");
            out.push_str(&format!("{name:<7} not available: {reason}\n"));
            if let Some(account) = &agent.account {
                out.push_str(&format!("        {}\n", usage::account_text(account)));
            }
            continue;
        };
        let mut lead = name;
        let rows = usage::window_rows(agent);
        let width = rows
            .iter()
            .map(|row| match row {
                usage::WindowRow::Reading(window) => usage::window_name(window),
                usage::WindowRow::ResetBefore(minutes) => usage::length_name(*minutes),
            })
            .map(|name| name.chars().count())
            .max()
            .unwrap_or(0)
            .max(5);
        for row in rows {
            let (name, share) = match row {
                usage::WindowRow::Reading(window)
                    if usage::has_reset(window, read_at, now_secs) =>
                {
                    (usage::window_name(window), usage::RESET_SINCE.to_owned())
                }
                usage::WindowRow::Reading(window) => {
                    let reset = window.resets_at.map_or_else(
                        || "reset time unknown".to_owned(),
                        |at| usage::reset_text(at, now),
                    );
                    let share = format!("{:>4}  {reset}", usage::percent_text(window.used_percent));
                    (usage::window_name(window), share)
                }
                usage::WindowRow::ResetBefore(minutes) => {
                    (usage::length_name(minutes), usage::RESET_BEFORE.to_owned())
                }
            };
            out.push_str(&format!("{lead:<7} {name:<width$} {share}\n"));
            lead = String::new();
        }
        if let Some(account) = &agent.account {
            out.push_str(&format!("        {}\n", usage::account_text(account)));
        }
        if let Some(note) = usage::update_note(agent, now) {
            match &agent.problem {
                Some(problem) => out.push_str(&format!("        {note}: {problem}\n")),
                None => out.push_str(&format!("        {note}\n")),
            }
        }
    }
    out
}

fn tally(total: usize, needing: usize) -> String {
    if needing > 0 {
        format!("{total} · {needing} needs you")
    } else {
        total.to_string()
    }
}

/// One session; `place` (its folder or group) comes before the title.
fn row(session: &SessionInfo, now_ms: u64, mark: &str, place: Option<&str>) -> String {
    let where_ = place.map_or_else(String::new, |place| format!("{place}  "));
    let line = format!(
        "{mark}{:<20} {:<6} {:<32} {:>4}  {where_}{}",
        session.name,
        session.agent,
        status_text(session),
        age(now_ms.saturating_sub(session.last_activity_at)),
        title(session),
    );
    format!("{}\n", line.trim_end())
}

fn status_text(session: &SessionInfo) -> String {
    if let Some(external) = &session.external {
        let state = external.status.map_or("running", Status::label);
        let pid = external
            .pid
            .map_or_else(String::new, |pid| format!(" {pid}"));
        return format!("{state} · external{pid}");
    }
    let base = session.status.label();
    match (session.status, session.needs_reason) {
        (Status::NeedsYou, Some(reason)) if reason != NeedsReason::Unknown => {
            format!("{base} ({})", reason_text(reason))
        }
        _ if session.interrupted => format!("{base} · interrupted"),
        _ => base.to_owned(),
    }
}

/// The agent's title, cut to fit one line.
fn title(session: &SessionInfo) -> String {
    let Some(title) = &session.title else {
        return String::new();
    };
    let title: String = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if title.chars().count() <= TITLE_WIDTH {
        return title;
    }
    let cut: String = title.chars().take(TITLE_WIDTH - 1).collect();
    format!("{cut}…")
}

fn reason_text(reason: NeedsReason) -> &'static str {
    match reason {
        NeedsReason::Permission => "permission",
        NeedsReason::Question => "question",
        NeedsReason::Auth => "sign-in",
        NeedsReason::Input => "input",
        NeedsReason::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hq_signals::event::Agent;

    fn session(id: &str, cwd: &str, name: &str, status: Status) -> SessionInfo {
        SessionInfo {
            id: id.to_owned(),
            agent: Agent::Claude,
            cwd: cwd.to_owned(),
            name: name.to_owned(),
            agent_session_id: None,
            status,
            needs_reason: (status == Status::NeedsYou).then_some(NeedsReason::Permission),
            created_at: 0,
            last_activity_at: 0,
            ended_at: None,
            live: true,
            origin: hq_protocol::Origin::Hosted,
            title: None,
            external: None,
            in_sidebar: true,
            group: None,
            owner_named: false,
            interrupted: false,
            offered: true,
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
    fn sidebar_lists_needs_you_then_groups_then_ungrouped() {
        let mut asking = session(
            "aaaaaaaa-1",
            "/x/work/app",
            "landing-hero",
            Status::NeedsYou,
        );
        asking.group = Some(1);
        let sessions = [
            asking,
            session("bbbbbbbb-2", "/x/work/app", "pricing", Status::Working),
            session("cccccccc-3", "/x/side/app", "notes", Status::Done),
        ];
        let groups = [group(1, "Launch"), group(2, "Empty")];
        let text = sidebar_list(&sessions, &groups, 0);
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines[0], "Needs you  1");
        assert!(lines[1].contains("landing-hero"), "{text}");
        assert!(lines[1].contains("needs you (permission)"), "{text}");
        assert!(lines[1].contains("work/app"), "{text}");
        assert_eq!(lines[2], "Launch  1 · 1 needs you");
        assert!(lines[3].contains("landing-hero"), "{text}");
        assert_eq!(lines[4], "Empty  0");
        assert_eq!(lines[5], "Ungrouped  2");
        assert!(text.contains("side/app"), "{text}");
        assert_eq!(
            text.matches("landing-hero").count(),
            2,
            "in Needs you and in its group"
        );
    }

    #[test]
    fn without_groups_the_sessions_have_no_heading() {
        let sessions = [session("a", "/x/api", "api", Status::Done)];
        let text = sidebar_list(&sessions, &[], 0);
        assert!(!text.contains("Ungrouped"), "{text}");
        assert!(text.starts_with("  api"), "{text}");
    }

    #[test]
    fn sidebar_leaves_out_sessions_not_added() {
        let mut outside = session("o", "/w", "old-chat", Status::Inactive);
        outside.in_sidebar = false;
        let text = sidebar_list(&[outside.clone()], &[], 0);
        assert!(text.contains("The sidebar is empty"), "{text}");
        assert!(
            text.contains("hq ls --all") && text.contains("hq add"),
            "{text}"
        );

        let mut added = session("a", "/w", "kept", Status::Done);
        added.group = Some(1);
        let catalog = catalog_list(&[outside, added], &[group(1, "Docs")], 0);
        assert!(catalog.contains("  old-chat"), "{catalog}");
        assert!(catalog.contains("* kept"), "{catalog}");
        assert!(catalog.contains("[Docs]"), "{catalog}");
    }

    #[test]
    fn interrupted_sessions_are_marked() {
        let mut stopped = session("a", "/w/api", "api-fix", Status::Inactive);
        stopped.in_sidebar = true;
        stopped.interrupted = true;
        let text = sidebar_list(&[stopped], &[], 0);
        assert!(text.contains("idle · interrupted"), "{text}");
    }

    #[test]
    fn external_status_is_shown() {
        let mut running = session("x", "/w", "outside", Status::Inactive);
        running.live = false;
        running.external = Some(hq_protocol::External {
            pid: Some(7),
            status: Some(Status::NeedsYou),
        });
        let text = sidebar_list(&[running], &[], 0);
        assert!(text.contains("needs you · external 7"), "{text}");
    }

    #[test]
    fn empty_catalog_explains_how_to_start() {
        assert!(catalog_list(&[], &[], 0).contains("hq new"));
    }

    #[test]
    fn titles_are_shown_and_cut_to_one_line() {
        let mut with_title = session("t", "/w", "claude-3f2a", Status::Done);
        with_title.title = Some("Fix the\nlogin   flow".into());
        assert!(sidebar_list(&[with_title.clone()], &[], 0).contains("Fix the login flow"));
        with_title.title = Some("x".repeat(60));
        let text = sidebar_list(&[with_title], &[], 0);
        assert!(text.contains(&format!("{}…", "x".repeat(TITLE_WIDTH - 1))));
    }

    #[test]
    fn usage_lists_windows_ages_and_reasons() {
        use chrono::TimeZone;
        let zone = chrono::FixedOffset::east_opt(7 * 3_600).unwrap();
        let now = zone.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap();
        let now_secs = u64::try_from(now.timestamp()).unwrap();
        let window = |minutes, used, resets_at| hq_protocol::UsageWindow {
            minutes: Some(minutes),
            used_percent: used,
            resets_at: Some(resets_at),
            model: None,
        };
        let agents = [
            AgentUsage {
                agent: Agent::Codex,
                windows: vec![
                    window(300, 82.4, now_secs + 5 * 3_600 + 40 * 60),
                    window(10_080, 20.0, now_secs + 3 * 86_400 - 3_600),
                ],
                read_at: Some(now_secs - 240),
                problem: None,
                account: Some(hq_protocol::Account::Subscription {
                    plan: Some("plus".into()),
                }),
            },
            AgentUsage {
                agent: Agent::Claude,
                windows: vec![window(300, 95.0, now_secs - 3_600)],
                read_at: Some(now_secs - 3 * 3_600),
                problem: Some("timed out".into()),
                account: None,
            },
            AgentUsage {
                agent: Agent::Agy,
                windows: vec![],
                read_at: None,
                problem: Some("Agy usage not available".into()),
                account: None,
            },
        ];
        let text = usage_list(&agents, &now);
        assert!(
            text.contains("codex   5h     82%  resets 3:40 PM"),
            "{text}"
        );
        assert!(
            text.contains("        week   20%  resets Fri 9:00 AM"),
            "{text}"
        );
        assert!(text.contains("        Plus plan\n"), "{text}");
        assert!(!text.contains("as of"), "{text}");
        assert!(
            text.contains("claude  5h    reset since this reading"),
            "{text}"
        );
        assert!(
            text.contains("        week  reset before this reading"),
            "{text}"
        );
        assert!(
            text.contains("        Couldn't update since 7:00 AM: timed out"),
            "{text}"
        );
        assert!(
            text.contains("agy     not available: Agy usage not available"),
            "{text}"
        );
    }
}
