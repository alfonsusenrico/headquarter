//! The sessions already in a folder: counted while browsing, and the ones not in the
//! sidebar offered before a new session starts there (add-folder-sessions D1, D2, D4).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use hq_protocol::{Origin, SessionInfo};

/// Whether a session counts as one of its folder's: in the sidebar, or a past
/// conversation Add sessions offers. One running in another terminal is left out.
fn counted(session: &SessionInfo) -> bool {
    session.origin != Origin::External
        && session.external.is_none()
        && !session.cwd.is_empty()
        && (session.in_sidebar || session.offered)
}

/// How many sessions each folder holds, by exact path.
pub fn counts(sessions: &[SessionInfo]) -> HashMap<PathBuf, usize> {
    let mut counts = HashMap::new();
    for session in sessions.iter().filter(|session| counted(session)) {
        *counts.entry(PathBuf::from(&session.cwd)).or_default() += 1;
    }
    counts
}

/// A folder row's count, or nothing for a folder without sessions.
pub fn count_label(count: usize) -> Option<String> {
    match count {
        0 => None,
        1 => Some("1 session".to_owned()),
        many => Some(format!("{many} sessions")),
    }
}

/// The sessions in `folder` that are not in the sidebar and that the owner did not ask
/// to leave out, most recently used first.
pub fn forgotten<'a>(
    sessions: &'a [SessionInfo],
    folder: &Path,
    not_asked: &HashSet<String>,
) -> Vec<&'a SessionInfo> {
    let mut offered: Vec<&SessionInfo> = sessions
        .iter()
        .filter(|session| counted(session) && !session.in_sidebar)
        .filter(|session| Path::new(&session.cwd) == folder)
        .filter(|session| !not_asked.contains(&session.id))
        .collect();
    offered.sort_by_key(|session| std::cmp::Reverse(session.last_activity_at));
    offered
}

/// The kept "don't ask again" list with `listed` added, keeping only sessions that
/// still exist.
pub fn not_asked_after(
    kept: &[String],
    listed: &[String],
    sessions: &[SessionInfo],
) -> Vec<String> {
    let existing: HashSet<&str> = sessions.iter().map(|session| session.id.as_str()).collect();
    let mut after: Vec<String> = Vec::new();
    for id in kept.iter().chain(listed) {
        if existing.contains(id.as_str()) && !after.contains(id) {
            after.push(id.clone());
        }
    }
    after
}

#[cfg(test)]
mod tests {
    use hq_protocol::External;
    use hq_signals::event::Agent;
    use hq_signals::status::Status;

    use super::*;

    fn session(id: &str, cwd: &str, in_sidebar: bool, last_activity_at: u64) -> SessionInfo {
        SessionInfo {
            id: id.to_owned(),
            agent: Agent::Claude,
            cwd: cwd.to_owned(),
            name: id.to_owned(),
            agent_session_id: None,
            status: Status::Inactive,
            needs_reason: None,
            created_at: 0,
            last_activity_at,
            ended_at: None,
            live: false,
            origin: if in_sidebar {
                Origin::Hosted
            } else {
                Origin::History
            },
            title: None,
            external: None,
            in_sidebar,
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
    fn folders_count_sidebar_sessions_and_past_conversations() {
        let mut elsewhere = session("elsewhere", "/u/web", false, 3);
        elsewhere.external = Some(External {
            pid: Some(7),
            status: None,
            background: false,
        });
        let mut outside = session("outside", "/u/web", false, 3);
        outside.origin = Origin::External;
        let mut gone = session("gone", "/u/web", false, 3);
        gone.offered = false;
        let sessions = vec![
            session("one", "/u/web", true, 1),
            session("two", "/u/web", false, 2),
            session("three", "/u/web", false, 4),
            session("api", "/u/api", true, 1),
            session("sub", "/u/web/src", false, 1),
            elsewhere,
            outside,
            gone,
        ];
        let counts = counts(&sessions);
        assert_eq!(counts.get(Path::new("/u/web")), Some(&3));
        assert_eq!(counts.get(Path::new("/u/api")), Some(&1));
        assert_eq!(counts.get(Path::new("/u/web/src")), Some(&1));
        assert_eq!(count_label(1).as_deref(), Some("1 session"));
        assert_eq!(count_label(3).as_deref(), Some("3 sessions"));
        assert_eq!(count_label(0), None);
    }

    #[test]
    fn only_sessions_not_in_the_sidebar_are_offered() {
        let none = HashSet::new();
        let web = Path::new("/u/web");
        let all_added = vec![session("one", "/u/web", true, 1)];
        assert!(forgotten(&all_added, web, &none).is_empty());

        let one_of_two = vec![
            session("one", "/u/web", true, 1),
            session("two", "/u/web", false, 2),
        ];
        let offered = forgotten(&one_of_two, web, &none);
        assert_eq!(offered.len(), 1);
        assert_eq!(offered[0].id, "two");

        let neither = vec![
            session("old", "/u/web", false, 1),
            session("new", "/u/web", false, 5),
            session("sub", "/u/web/src", false, 9),
        ];
        let ids: Vec<&str> = forgotten(&neither, web, &none)
            .iter()
            .map(|session| session.id.as_str())
            .collect();
        assert_eq!(ids, ["new", "old"], "most recent first, subfolders apart");

        let not_asked = HashSet::from(["two".to_owned()]);
        assert!(forgotten(&one_of_two, web, &not_asked).is_empty());
    }

    #[test]
    fn the_not_asked_list_grows_and_forgets_sessions_that_are_gone() {
        let sessions = vec![
            session("kept", "/u/web", false, 1),
            session("listed", "/u/web", false, 2),
        ];
        let after = not_asked_after(
            &["gone".to_owned(), "kept".to_owned()],
            &["listed".to_owned(), "kept".to_owned()],
            &sessions,
        );
        assert_eq!(after, ["kept", "listed"]);
    }
}
