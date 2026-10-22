//! Headquarter's session index: SQLite with embedded, forward-only migrations whose
//! checksums are recorded, so an applied migration cannot change silently.
//!
//! The index holds metadata only (see `migrations/`); terminal output, prompts, and
//! model output are never stored.

use std::path::Path;

use hq_signals::event::{Agent, NeedsReason};
use hq_signals::status::Status;
use rusqlite::{Connection, OptionalExtension, Row, params};
use sha2::{Digest, Sha256};

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "sessions",
        sql: include_str!("../migrations/0001_sessions.sql"),
    },
    Migration {
        version: 2,
        name: "history",
        sql: include_str!("../migrations/0002_history.sql"),
    },
    Migration {
        version: 3,
        name: "sidebar",
        sql: include_str!("../migrations/0003_sidebar.sql"),
    },
    Migration {
        version: 4,
        name: "groups",
        sql: include_str!("../migrations/0004_groups.sql"),
    },
    Migration {
        version: 5,
        name: "owner_named",
        sql: include_str!("../migrations/0005_owner_named.sql"),
    },
    Migration {
        version: 6,
        name: "interrupted",
        sql: include_str!("../migrations/0006_interrupted.sql"),
    },
];

/// How a session entered the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Started by Headquarter.
    Hosted,
    /// Imported from an agent's own history.
    History,
}

impl Origin {
    pub fn text(self) -> &'static str {
        match self {
            Origin::Hosted => "hosted",
            Origin::History => "history",
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    Sqlite(rusqlite::Error),
    /// An applied migration's file no longer matches what was applied.
    ChangedMigration {
        version: i64,
        name: String,
    },
    /// A stored value the code does not recognise (written by a newer version).
    UnknownValue {
        column: &'static str,
        value: String,
    },
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Sqlite(error) => write!(f, "database error: {error}"),
            StoreError::ChangedMigration { version, name } => write!(
                f,
                "applied migration {version} ({name}) has changed since it was applied"
            ),
            StoreError::UnknownValue { column, value } => {
                write!(f, "unknown {column} value `{value}` in the database")
            }
        }
    }
}

impl std::error::Error for StoreError {}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        StoreError::Sqlite(error)
    }
}

pub type Result<T> = std::result::Result<T, StoreError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    pub id: String,
    pub agent: Agent,
    pub cwd: String,
    pub name: String,
    pub agent_session_id: Option<String>,
    pub status: Status,
    pub needs_reason: Option<NeedsReason>,
    /// Unix milliseconds.
    pub created_at: u64,
    pub last_activity_at: u64,
    pub ended_at: Option<u64>,
    pub origin: Origin,
    /// When the session joined the owner's sidebar; `None` when it is not in it.
    pub sidebar_added_at: Option<u64>,
    /// The owner's group holding the session; `None` when it is in no group.
    pub group_id: Option<i64>,
    /// Whether `name` is one the owner gave, shown instead of the agent's title.
    pub owner_named: bool,
    /// When start-up recovery found the session still running; `None` otherwise.
    pub interrupted_at: Option<u64>,
}

/// A group the owner made in the sidebar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRow {
    pub id: i64,
    pub name: String,
    /// The group's place among the groups, from 0.
    pub position: i64,
    pub collapsed: bool,
}

pub struct Store {
    conn: Connection,
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        Store::prepare(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Store> {
        Store::prepare(Connection::open_in_memory()?)
    }

    fn prepare(mut conn: Connection) -> Result<Store> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA busy_timeout = 5000;",
        )?;
        migrate(&mut conn, MIGRATIONS)?;
        Ok(Store { conn })
    }

    pub fn insert(&self, row: &SessionRow) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sessions (id, agent, cwd, name, agent_session_id, status, needs_reason,
                                   created_at, last_activity_at, ended_at, origin,
                                   sidebar_added_at, group_id, owner_named)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                row.id,
                row.agent.id(),
                row.cwd,
                row.name,
                row.agent_session_id,
                status_text(row.status),
                row.needs_reason.map(reason_text),
                millis(row.created_at),
                millis(row.last_activity_at),
                row.ended_at.map(millis),
                row.origin.text(),
                row.sidebar_added_at.map(millis),
                row.group_id,
                row.owner_named,
            ],
        )?;
        Ok(())
    }

    /// The session holding an agent's conversation, if any.
    pub fn find_by_conversation(
        &self,
        agent: Agent,
        conversation: &str,
    ) -> Result<Option<SessionRow>> {
        self.conn
            .query_row(
                &format!(
                    "{SELECT_SESSIONS} WHERE agent = ?1 AND agent_session_id = ?2
                     ORDER BY last_activity_at DESC LIMIT 1"
                ),
                params![agent.id(), conversation],
                read_row,
            )
            .optional()?
            .transpose()
    }

    /// Deletes an imported row; rows Headquarter started are never deleted.
    pub fn remove_history(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM sessions WHERE id = ?1 AND origin = 'history'",
            params![id],
        )?;
        Ok(())
    }

    /// Moves a session's last activity forward; earlier times are ignored.
    pub fn advance_activity(&self, id: &str, at: u64) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET last_activity_at = MAX(last_activity_at, ?2) WHERE id = ?1",
            params![id, millis(at)],
        )?;
        Ok(())
    }

    /// Records a status; `activity` moves the last activity time when the change was
    /// the agent's doing, and `None` keeps it.
    pub fn update_status(
        &self,
        id: &str,
        status: Status,
        reason: Option<NeedsReason>,
        activity: Option<u64>,
    ) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET status = ?2, needs_reason = ?3,
                                 last_activity_at = COALESCE(?4, last_activity_at)
             WHERE id = ?1",
            params![
                id,
                status_text(status),
                reason.map(reason_text),
                activity.map(millis)
            ],
        )?;
        Ok(())
    }

    pub fn set_agent_session_id(&self, id: &str, agent_session_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET agent_session_id = ?2 WHERE id = ?1",
            params![id, agent_session_id],
        )?;
        Ok(())
    }

    /// Renames a session; `owner_named` says whether the owner gave the name.
    pub fn rename(&self, id: &str, name: &str, owner_named: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET name = ?2, owner_named = ?3 WHERE id = ?1",
            params![id, name, owner_named],
        )?;
        Ok(())
    }

    /// Marks a session inactive because its agent ended.
    pub fn end(&self, id: &str, at: u64) -> Result<()> {
        // The last activity stays: an agent stopping (often with the host) is not work
        // the owner sees, and the session shows as idle from then on.
        self.conn.execute(
            "UPDATE sessions SET status = 'inactive', needs_reason = NULL, ended_at = ?2
             WHERE id = ?1",
            params![id, millis(at)],
        )?;
        Ok(())
    }

    /// Makes an inactive session live again for a resume. Its last activity stays:
    /// opening a session is not something its agent did.
    pub fn reactivate(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET status = 'done', needs_reason = NULL, ended_at = NULL,
                                 interrupted_at = NULL
             WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Start-up recovery: ends every session still marked running and marks it
    /// interrupted at `at`, since the host before stopped without ending it (a crash, a
    /// reboot, a kill). No earlier agent can still be running: the host owns the PTYs.
    /// Returns how many changed.
    pub fn recover_interrupted(&self, at: u64) -> Result<usize> {
        Ok(self.conn.execute(
            "UPDATE sessions SET status = 'inactive', needs_reason = NULL, ended_at = ?1,
                                 interrupted_at = ?1
             WHERE status <> 'inactive'",
            params![millis(at)],
        )?)
    }

    /// Clears the interrupted mark of `ids` without restarting them; an empty list clears
    /// every mark. Returns how many changed.
    pub fn dismiss_interrupted(&self, ids: &[String]) -> Result<usize> {
        if ids.is_empty() {
            return Ok(self.conn.execute(
                "UPDATE sessions SET interrupted_at = NULL WHERE interrupted_at IS NOT NULL",
                [],
            )?);
        }
        let mut changed = 0;
        for id in ids {
            changed += self.conn.execute(
                "UPDATE sessions SET interrupted_at = NULL WHERE id = ?1",
                params![id],
            )?;
        }
        Ok(changed)
    }

    /// Marks every session not already inactive as ended at `at`, without marking it
    /// interrupted: the host is stopping on purpose after ending its agents. Returns how
    /// many changed.
    pub fn end_all_live(&self, at: u64) -> Result<usize> {
        Ok(self.conn.execute(
            "UPDATE sessions SET status = 'inactive', needs_reason = NULL, ended_at = ?1
             WHERE status <> 'inactive'",
            params![millis(at)],
        )?)
    }

    pub fn get(&self, id: &str) -> Result<Option<SessionRow>> {
        self.conn
            .query_row(
                &format!("{SELECT_SESSIONS} WHERE id = ?1"),
                params![id],
                read_row,
            )
            .optional()?
            .transpose()
    }

    /// Every session, grouped by folder and most recent first within a folder.
    pub fn list(&self) -> Result<Vec<SessionRow>> {
        let mut statement = self.conn.prepare(&format!(
            "{SELECT_SESSIONS} ORDER BY cwd, last_activity_at DESC"
        ))?;
        let rows = statement.query_map([], read_row)?;
        let mut sessions = Vec::new();
        for row in rows {
            sessions.push(row??);
        }
        Ok(sessions)
    }

    /// Puts sessions in the sidebar at `at`; sessions already there keep their time.
    /// All or nothing.
    pub fn add_to_sidebar(&self, ids: &[String], at: u64) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for id in ids {
            tx.execute(
                "UPDATE sessions SET sidebar_added_at = COALESCE(sidebar_added_at, ?2)
                 WHERE id = ?1",
                params![id, millis(at)],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Takes sessions out of the sidebar and out of their groups; the rows stay. All or
    /// nothing.
    pub fn remove_from_sidebar(&self, ids: &[String]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for id in ids {
            tx.execute(
                "UPDATE sessions SET sidebar_added_at = NULL, group_id = NULL WHERE id = ?1",
                params![id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    /// The owner's groups, in the owner's order.
    pub fn groups(&self) -> Result<Vec<GroupRow>> {
        let mut statement = self
            .conn
            .prepare("SELECT id, name, position, collapsed FROM groups ORDER BY position, id")?;
        let groups = statement
            .query_map([], |row| {
                Ok(GroupRow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    position: row.get(2)?,
                    collapsed: row.get::<_, i64>(3)? == 1,
                })
            })?
            .collect::<rusqlite::Result<Vec<GroupRow>>>()?;
        Ok(groups)
    }

    pub fn group(&self, id: i64) -> Result<Option<GroupRow>> {
        Ok(self.groups()?.into_iter().find(|group| group.id == id))
    }

    /// Adds a group after the others and returns its id.
    pub fn create_group(&self, name: &str, at: u64) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO groups (name, position, created_at)
             VALUES (?1, (SELECT COALESCE(MAX(position) + 1, 0) FROM groups), ?2)",
            params![name, millis(at)],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn rename_group(&self, id: i64, name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE groups SET name = ?2 WHERE id = ?1",
            params![id, name],
        )?;
        Ok(())
    }

    /// Deletes a group; its sessions stay, in no group (the foreign key's `SET NULL`).
    pub fn delete_group(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM groups WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Moves a group to just before `before`, or to the end. Every position is
    /// rewritten in one transaction, so positions stay 0, 1, 2, ...
    pub fn move_group(&self, id: i64, before: Option<i64>) -> Result<()> {
        let mut order: Vec<i64> = self
            .groups()?
            .into_iter()
            .map(|group| group.id)
            .filter(|known| *known != id)
            .collect();
        let at = before
            .and_then(|before| order.iter().position(|known| *known == before))
            .unwrap_or(order.len());
        order.insert(at, id);
        let tx = self.conn.unchecked_transaction()?;
        for (position, group) in order.iter().enumerate() {
            tx.execute(
                "UPDATE groups SET position = ?2 WHERE id = ?1",
                params![group, position as i64],
            )?;
        }
        tx.commit()?;
        Ok(())
    }

    pub fn set_group_collapsed(&self, id: i64, collapsed: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE groups SET collapsed = ?2 WHERE id = ?1",
            params![id, i64::from(collapsed)],
        )?;
        Ok(())
    }

    /// Puts a session in a group, or in none.
    pub fn set_session_group(&self, session: &str, group: Option<i64>) -> Result<()> {
        self.conn.execute(
            "UPDATE sessions SET group_id = ?2 WHERE id = ?1",
            params![session, group],
        )?;
        Ok(())
    }

    /// Names are unique across all sessions so `hq attach <name>` is never
    /// ambiguous.
    pub fn name_taken(&self, name: &str) -> Result<bool> {
        Ok(self
            .conn
            .prepare("SELECT 1 FROM sessions WHERE name = ?1")?
            .exists(params![name])?)
    }

    /// The first free `<agent>-<n>` name.
    pub fn next_default_name(&self, agent: Agent) -> Result<String> {
        for n in 1.. {
            let name = format!("{}-{n}", agent.id());
            if !self.name_taken(&name)? {
                return Ok(name);
            }
        }
        unreachable!("an unbounded counter always finds a free name")
    }
}

const SELECT_SESSIONS: &str = "SELECT id, agent, cwd, name, agent_session_id, status, needs_reason,
                                      created_at, last_activity_at, ended_at, origin,
                                      sidebar_added_at, group_id, owner_named, interrupted_at
                               FROM sessions";

fn read_row(row: &Row<'_>) -> rusqlite::Result<Result<SessionRow>> {
    let agent: String = row.get(1)?;
    let status: String = row.get(5)?;
    let reason: Option<String> = row.get(6)?;
    let created_at: i64 = row.get(7)?;
    let last_activity_at: i64 = row.get(8)?;
    let ended_at: Option<i64> = row.get(9)?;
    let origin: String = row.get(10)?;
    let sidebar_added_at: Option<i64> = row.get(11)?;
    let group_id: Option<i64> = row.get(12)?;
    let owner_named: bool = row.get(13)?;
    let interrupted_at: Option<i64> = row.get(14)?;
    let id: String = row.get(0)?;
    let cwd: String = row.get(2)?;
    let name: String = row.get(3)?;
    let agent_session_id: Option<String> = row.get(4)?;
    Ok((|| {
        Ok(SessionRow {
            id,
            agent: Agent::from_id(&agent).ok_or(StoreError::UnknownValue {
                column: "agent",
                value: agent.clone(),
            })?,
            cwd,
            name,
            agent_session_id,
            status: parse_status(&status)?,
            needs_reason: reason.as_deref().map(parse_reason).transpose()?,
            created_at: unmillis(created_at),
            last_activity_at: unmillis(last_activity_at),
            ended_at: ended_at.map(unmillis),
            origin: match origin.as_str() {
                "hosted" => Origin::Hosted,
                "history" => Origin::History,
                _ => {
                    return Err(StoreError::UnknownValue {
                        column: "origin",
                        value: origin,
                    });
                }
            },
            sidebar_added_at: sidebar_added_at.map(unmillis),
            group_id,
            owner_named,
            interrupted_at: interrupted_at.map(unmillis),
        })
    })())
}

fn millis(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn unmillis(value: i64) -> u64 {
    u64::try_from(value).unwrap_or(0)
}

pub fn status_text(status: Status) -> &'static str {
    match status {
        Status::Working => "working",
        Status::NeedsYou => "needs-you",
        Status::DoneUnread => "done-unread",
        Status::Done => "done",
        Status::Inactive => "inactive",
    }
}

fn parse_status(text: &str) -> Result<Status> {
    match text {
        "working" => Ok(Status::Working),
        "needs-you" => Ok(Status::NeedsYou),
        "done-unread" => Ok(Status::DoneUnread),
        "done" => Ok(Status::Done),
        "inactive" => Ok(Status::Inactive),
        _ => Err(StoreError::UnknownValue {
            column: "status",
            value: text.to_owned(),
        }),
    }
}

fn reason_text(reason: NeedsReason) -> &'static str {
    match reason {
        NeedsReason::Permission => "permission",
        NeedsReason::Question => "question",
        NeedsReason::Auth => "auth",
        NeedsReason::Input => "input",
        NeedsReason::Unknown => "unknown",
    }
}

fn parse_reason(text: &str) -> Result<NeedsReason> {
    match text {
        "permission" => Ok(NeedsReason::Permission),
        "question" => Ok(NeedsReason::Question),
        "auth" => Ok(NeedsReason::Auth),
        "input" => Ok(NeedsReason::Input),
        "unknown" => Ok(NeedsReason::Unknown),
        _ => Err(StoreError::UnknownValue {
            column: "needs_reason",
            value: text.to_owned(),
        }),
    }
}

fn checksum(sql: &str) -> String {
    Sha256::digest(sql.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Applies each migration not yet recorded, in its own transaction, and refuses to
/// continue when a recorded migration's text has changed.
pub fn migrate(conn: &mut Connection, migrations: &[Migration]) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
             version    INTEGER PRIMARY KEY,
             name       TEXT NOT NULL,
             checksum   TEXT NOT NULL,
             applied_at INTEGER NOT NULL
         )",
    )?;
    for migration in migrations {
        let expected = checksum(migration.sql);
        let applied: Option<String> = conn
            .query_row(
                "SELECT checksum FROM schema_migrations WHERE version = ?1",
                params![migration.version],
                |row| row.get(0),
            )
            .optional()?;
        match applied {
            Some(recorded) if recorded == expected => continue,
            Some(_) => {
                return Err(StoreError::ChangedMigration {
                    version: migration.version,
                    name: migration.name.to_owned(),
                });
            }
            None => {
                let tx = conn.transaction()?;
                tx.execute_batch(migration.sql)?;
                tx.execute(
                    "INSERT INTO schema_migrations (version, name, checksum, applied_at)
                     VALUES (?1, ?2, ?3, ?4)",
                    params![
                        migration.version,
                        migration.name,
                        expected,
                        millis(hq_signals::now_ms())
                    ],
                )?;
                tx.commit()?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(id: &str, cwd: &str, name: &str) -> SessionRow {
        SessionRow {
            id: id.to_owned(),
            agent: Agent::Claude,
            cwd: cwd.to_owned(),
            name: name.to_owned(),
            agent_session_id: None,
            status: Status::Done,
            needs_reason: None,
            created_at: 1_000,
            last_activity_at: 1_000,
            ended_at: None,
            origin: Origin::Hosted,
            sidebar_added_at: None,
            group_id: None,
            owner_named: false,
            interrupted_at: None,
        }
    }

    #[test]
    fn migrations_apply_once_and_rerun_is_a_no_op() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, i64::try_from(MIGRATIONS.len()).unwrap());
        // Migrations written with IF NOT EXISTS are idempotent as SQL too. 0002 adds a
        // column (as does 0003), which SQLite cannot guard in SQL; the recorded-migration check above
        // is what keeps it from running twice.
        conn.execute_batch(MIGRATIONS[0].sql).unwrap();
    }

    #[test]
    fn changed_applied_migration_is_refused() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        let edited = [Migration {
            version: 1,
            name: "sessions",
            sql: "CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY);",
        }];
        assert!(matches!(
            migrate(&mut conn, &edited),
            Err(StoreError::ChangedMigration { version: 1, .. })
        ));
    }

    #[test]
    fn schema_has_only_metadata_columns() {
        let store = Store::open_in_memory().unwrap();
        let mut statement = store
            .conn
            .prepare("SELECT name FROM pragma_table_info('sessions')")
            .unwrap();
        let columns: Vec<String> = statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|column| column.unwrap())
            .collect();
        assert_eq!(
            columns,
            [
                "id",
                "agent",
                "cwd",
                "name",
                "agent_session_id",
                "status",
                "needs_reason",
                "created_at",
                "last_activity_at",
                "ended_at",
                "origin",
                "sidebar_added_at",
                "group_id",
                "owner_named",
                "interrupted_at"
            ]
        );
    }

    #[test]
    fn recovery_marks_what_was_running_and_restarting_or_dismissing_clears_it() {
        let store = Store::open_in_memory().unwrap();
        store.insert(&row("a", "/w", "claude-1")).unwrap();
        store.insert(&row("b", "/w", "claude-2")).unwrap();
        store.insert(&row("c", "/w", "claude-3")).unwrap();
        store.end("c", 1_000).unwrap();
        assert_eq!(store.recover_interrupted(5_000).unwrap(), 2);
        let marked = |id: &str| store.get(id).unwrap().unwrap().interrupted_at;
        assert_eq!(marked("a"), Some(5_000));
        assert_eq!(marked("c"), None, "already ended before the host went away");
        store.reactivate("a").unwrap();
        assert_eq!(marked("a"), None, "restarting clears the mark");
        assert_eq!(store.dismiss_interrupted(&["b".to_owned()]).unwrap(), 1);
        assert_eq!(marked("b"), None);

        store.end_all_live(9_000).unwrap();
        assert_eq!(marked("a"), None, "a graceful stop marks nothing");
        store.recover_interrupted(9_500).unwrap();
        assert_eq!(
            store.dismiss_interrupted(&[]).unwrap(),
            0,
            "nothing was running"
        );
    }

    #[test]
    fn insert_get_and_update() {
        let store = Store::open_in_memory().unwrap();
        store.insert(&row("a", "/w", "claude-1")).unwrap();
        store
            .update_status(
                "a",
                Status::NeedsYou,
                Some(NeedsReason::Permission),
                Some(2_000),
            )
            .unwrap();
        store.set_agent_session_id("a", "conv-1").unwrap();
        store.rename("a", "landing-hero", true).unwrap();
        let got = store.get("a").unwrap().unwrap();
        assert_eq!(got.status, Status::NeedsYou);
        assert_eq!(got.needs_reason, Some(NeedsReason::Permission));
        assert_eq!(got.last_activity_at, 2_000);
        store.update_status("a", Status::Done, None, None).unwrap();
        let read = store.get("a").unwrap().unwrap();
        assert_eq!(
            (read.status, read.last_activity_at),
            (Status::Done, 2_000),
            "a change that is not activity keeps the time"
        );
        assert_eq!(got.agent_session_id.as_deref(), Some("conv-1"));
        assert_eq!(got.name, "landing-hero");
        assert!(got.owner_named);
        assert_eq!(store.get("missing").unwrap(), None);
    }

    #[test]
    fn end_reactivate_and_restart_recovery() {
        let store = Store::open_in_memory().unwrap();
        store.insert(&row("a", "/w", "claude-1")).unwrap();
        store.insert(&row("b", "/w", "claude-2")).unwrap();
        let before = store.get("a").unwrap().unwrap().last_activity_at;
        store.end("a", 5_000).unwrap();
        let ended = store.get("a").unwrap().unwrap();
        assert_eq!(ended.status, Status::Inactive);
        assert_eq!(ended.ended_at, Some(5_000));
        assert_eq!(ended.last_activity_at, before, "stopping is not activity");

        store.reactivate("a").unwrap();
        let live = store.get("a").unwrap().unwrap();
        assert_eq!(live.status, Status::Done);
        assert_eq!(live.ended_at, None);
        assert_eq!(
            live.last_activity_at, ended.last_activity_at,
            "opening is not activity"
        );

        assert_eq!(store.end_all_live(7_000).unwrap(), 2);
        assert!(
            store
                .list()
                .unwrap()
                .iter()
                .all(|session| session.status == Status::Inactive)
        );
    }

    #[test]
    fn list_is_grouped_by_folder_newest_first() {
        let store = Store::open_in_memory().unwrap();
        let mut older = row("a", "/b", "x");
        older.last_activity_at = 1;
        let mut newer = row("b", "/b", "y");
        newer.last_activity_at = 9;
        store.insert(&older).unwrap();
        store.insert(&newer).unwrap();
        store.insert(&row("c", "/a", "z")).unwrap();
        let ids: Vec<String> = store
            .list()
            .unwrap()
            .into_iter()
            .map(|session| session.id)
            .collect();
        assert_eq!(ids, ["c", "b", "a"]);
    }

    #[test]
    fn default_names_are_unique_across_folders() {
        let store = Store::open_in_memory().unwrap();
        assert_eq!(store.next_default_name(Agent::Claude).unwrap(), "claude-1");
        store.insert(&row("a", "/w", "claude-1")).unwrap();
        assert_eq!(store.next_default_name(Agent::Claude).unwrap(), "claude-2");
        store.insert(&row("b", "/other", "claude-2")).unwrap();
        assert_eq!(store.next_default_name(Agent::Claude).unwrap(), "claude-3");
        assert_eq!(store.next_default_name(Agent::Codex).unwrap(), "codex-1");
        assert!(store.name_taken("claude-2").unwrap());
        assert!(!store.name_taken("landing-hero").unwrap());
    }

    #[test]
    fn existing_rows_become_hosted_after_the_history_migration() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &MIGRATIONS[..1]).unwrap();
        conn.execute(
            "INSERT INTO sessions (id, agent, cwd, name, status, created_at, last_activity_at)
             VALUES ('old', 'claude', '/w', 'claude-1', 'done', 1, 1)",
            [],
        )
        .unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        let store = Store { conn };
        assert_eq!(store.get("old").unwrap().unwrap().origin, Origin::Hosted);
    }

    #[test]
    fn names_given_before_the_owner_named_migration_are_kept_as_the_owners() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &MIGRATIONS[..4]).unwrap();
        for (id, agent, name) in [
            ("generated", "claude", "claude-3"),
            ("history", "codex", "codex-1a2b3c4d"),
            ("renamed", "claude", "Landing hero"),
        ] {
            conn.execute(
                "INSERT INTO sessions (id, agent, cwd, name, status, created_at, last_activity_at)
                 VALUES (?1, ?2, '/w', ?3, 'done', 1, 1)",
                params![id, agent, name],
            )
            .unwrap();
        }
        migrate(&mut conn, MIGRATIONS).unwrap();
        let store = Store { conn };
        let named = |id: &str| store.get(id).unwrap().unwrap().owner_named;
        assert_eq!(
            (named("generated"), named("history"), named("renamed")),
            (false, false, true)
        );
    }

    #[test]
    fn conversations_are_found_and_only_move_forward() {
        let store = Store::open_in_memory().unwrap();
        let mut imported = row("h", "/w", "claude-abc");
        imported.agent_session_id = Some("conv-1".into());
        imported.origin = Origin::History;
        imported.status = Status::Inactive;
        store.insert(&imported).unwrap();
        let found = store
            .find_by_conversation(Agent::Claude, "conv-1")
            .unwrap()
            .unwrap();
        assert_eq!(found.origin, Origin::History);
        assert_eq!(
            store.find_by_conversation(Agent::Codex, "conv-1").unwrap(),
            None
        );

        let mut hosted = row("k", "/w", "claude-1");
        hosted.agent_session_id = Some("conv-2".into());
        store.insert(&hosted).unwrap();
        store.remove_history("k").unwrap();
        assert!(
            store.get("k").unwrap().is_some(),
            "hosted rows are never removed"
        );

        store.advance_activity("h", 5_000).unwrap();
        store.advance_activity("h", 2_000).unwrap();
        let moved = store.get("h").unwrap().unwrap();
        assert_eq!(moved.last_activity_at, 5_000);
        assert_eq!(moved.status, Status::Inactive);
    }

    #[test]
    fn file_database_survives_reopen() {
        let path = std::env::temp_dir().join(format!("hq-store-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&path);
        {
            let store = Store::open(&path).unwrap();
            store.insert(&row("a", "/w", "claude-1")).unwrap();
        }
        let store = Store::open(&path).unwrap();
        assert_eq!(store.list().unwrap().len(), 1);
        drop(store);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }

    #[test]
    fn upgrade_puts_hosted_sessions_in_the_sidebar() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &MIGRATIONS[..2]).unwrap();
        conn.execute_batch(
            "INSERT INTO sessions (id, agent, cwd, name, status, created_at, last_activity_at, origin)
             VALUES ('hosted', 'claude', '/w', 'claude-1', 'done', 7, 9, 'hosted'),
                    ('past', 'codex', '/w', 'codex-abc', 'inactive', 3, 3, 'history');",
        )
        .unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        let store = Store { conn };
        assert_eq!(
            store.get("hosted").unwrap().unwrap().sidebar_added_at,
            Some(7)
        );
        assert_eq!(store.get("past").unwrap().unwrap().sidebar_added_at, None);
    }

    #[test]
    fn sidebar_membership() {
        let store = Store::open_in_memory().unwrap();
        store.insert(&row("a", "/w", "claude-1")).unwrap();
        store.insert(&row("b", "/w", "claude-2")).unwrap();
        let both = ["a".to_owned(), "b".to_owned()];
        store.add_to_sidebar(&both, 5_000).unwrap();
        store.add_to_sidebar(&both[..1], 9_000).unwrap();
        assert_eq!(
            store.get("a").unwrap().unwrap().sidebar_added_at,
            Some(5_000),
            "adding again keeps the first time"
        );
        store.remove_from_sidebar(&both[1..]).unwrap();
        assert_eq!(store.get("b").unwrap().unwrap().sidebar_added_at, None);
        assert!(store.get("b").unwrap().is_some(), "removing keeps the row");
    }

    #[test]
    fn groups_order_collapse_and_membership() {
        let store = Store::open_in_memory().unwrap();
        store.insert(&row("a", "/w", "claude-1")).unwrap();
        store.add_to_sidebar(&["a".to_owned()], 1).unwrap();
        let backend = store.create_group("Backend", 1).unwrap();
        let docs = store.create_group("Docs", 2).unwrap();
        let web = store.create_group("Web", 3).unwrap();
        let names = |store: &Store| -> Vec<String> {
            store
                .groups()
                .unwrap()
                .into_iter()
                .map(|group| group.name)
                .collect()
        };
        assert_eq!(names(&store), ["Backend", "Docs", "Web"]);

        store.move_group(web, Some(backend)).unwrap();
        assert_eq!(names(&store), ["Web", "Backend", "Docs"]);
        store.move_group(web, None).unwrap();
        assert_eq!(names(&store), ["Backend", "Docs", "Web"]);
        let positions: Vec<i64> = store.groups().unwrap().iter().map(|g| g.position).collect();
        assert_eq!(positions, [0, 1, 2]);

        store.rename_group(docs, "Guides").unwrap();
        store.set_group_collapsed(docs, true).unwrap();
        let guides = store.group(docs).unwrap().unwrap();
        assert_eq!((guides.name.as_str(), guides.collapsed), ("Guides", true));

        store.set_session_group("a", Some(docs)).unwrap();
        assert_eq!(store.get("a").unwrap().unwrap().group_id, Some(docs));
        store.delete_group(docs).unwrap();
        assert_eq!(
            store.get("a").unwrap().unwrap().group_id,
            None,
            "deleting a group ungroups its sessions"
        );
        assert!(store.get("a").unwrap().unwrap().sidebar_added_at.is_some());

        store.set_session_group("a", Some(web)).unwrap();
        store.remove_from_sidebar(&["a".to_owned()]).unwrap();
        assert_eq!(
            store.get("a").unwrap().unwrap().group_id,
            None,
            "leaving the sidebar leaves the group"
        );
    }

    #[test]
    fn upgrade_from_folders_starts_everything_ungrouped() {
        let mut conn = Connection::open_in_memory().unwrap();
        migrate(&mut conn, &MIGRATIONS[..3]).unwrap();
        conn.execute_batch(
            "INSERT INTO sessions (id, agent, cwd, name, status, created_at, last_activity_at, origin)
             VALUES ('s', 'claude', '/w', 'claude-1', 'done', 7, 9, 'hosted');
             INSERT INTO folders (cwd, collapsed) VALUES ('/w', 1);",
        )
        .unwrap();
        migrate(&mut conn, MIGRATIONS).unwrap();
        let folders: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'folders'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(folders, 0);
        let store = Store { conn };
        assert_eq!(store.get("s").unwrap().unwrap().group_id, None);
        assert!(store.groups().unwrap().is_empty());
    }
}
