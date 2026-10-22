//! Importing the agents' own conversation history into the index.
//!
//! Titles go to an in-memory map for display and are never written to the index or
//! the log (owner decision 2026-10-20).

use std::collections::HashMap;
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use hq_signals::event::Agent;
use hq_signals::history::Conversation;
use hq_signals::status::Status;
use hq_store::{Origin, SessionRow, Store, StoreError};

use super::{Host, lock};
use crate::session::Hosted;

impl Host {
    pub(super) fn history_loop(self: Arc<Self>) {
        loop {
            self.import_history();
            thread::sleep(self.history_interval);
        }
    }

    fn import_history(&self) {
        let started = Instant::now();
        let report = lock(&self.history).scan();
        let mut titles = HashMap::new();
        let mut busy_agy = Vec::new();
        let mut added = 0;
        let mut moved = Vec::new();
        let mut failed = 0;
        for conversation in &report.conversations {
            if let Some(title) = &conversation.title {
                titles.insert((conversation.agent, conversation.id.clone()), title.clone());
            }
            if conversation.agent == Agent::Agy && conversation.busy {
                busy_agy.push((conversation.cwd.clone(), conversation.id.clone()));
            }
            let store = lock(&self.store);
            match merge(&store, conversation) {
                Ok(Merged::Added(id)) => {
                    added += 1;
                    moved.push(id);
                }
                Ok(Merged::Advanced(id)) => moved.push(id),
                Ok(Merged::Unchanged) => {}
                Err(_) => failed += 1,
            }
        }
        let retitled_conversations: Vec<(Agent, String)> = {
            let mut known = lock(&self.titles);
            let changed = titles
                .iter()
                .filter(|(conversation, title)| known.get(*conversation) != Some(*title))
                .map(|(conversation, _)| conversation.clone())
                .chain(
                    known
                        .keys()
                        .filter(|conversation| !titles.contains_key(*conversation))
                        .cloned(),
                )
                .collect();
            *known = titles;
            changed
        };
        let retitled: Vec<String> = {
            let store = lock(&self.store);
            retitled_conversations
                .iter()
                .filter_map(|(agent, id)| store.find_by_conversation(*agent, id).ok().flatten())
                .map(|row| row.id)
                .filter(|id| !moved.contains(id))
                .collect()
        };
        *lock(&self.busy_agy) = busy_agy;
        self.log.line(&format!(
            "history import conversations={} added={added} updated={} parsed={} skipped={} failed={failed} took_ms={}",
            report.conversations.len(),
            moved.len() - added,
            report.parsed,
            report.skipped,
            started.elapsed().as_millis()
        ));
        // A title can change while the conversation's activity does not, and clients
        // that listed before this pass have none yet.
        for id in moved.iter().chain(&retitled) {
            self.broadcast(id);
        }
    }

    /// A hosted Codex or agy session learns its conversation id only after its first
    /// turn; if an import listed that conversation first, the imported row goes.
    pub(super) fn drop_imported_duplicate(&self, hosted: &Hosted, conversation: &str) {
        let store = lock(&self.store);
        let Ok(Some(row)) = store.find_by_conversation(hosted.agent, conversation) else {
            return;
        };
        if row.id != hosted.id && row.origin == Origin::History {
            match store.remove_history(&row.id) {
                Ok(()) => self
                    .log
                    .line(&format!("merged imported duplicate into id={}", hosted.id)),
                Err(error) => self
                    .log
                    .line(&format!("index update failed id={} error={error}", row.id)),
            }
        }
    }
}

enum Merged {
    Added(String),
    Advanced(String),
    Unchanged,
}

fn merge(store: &Store, conversation: &Conversation) -> Result<Merged, StoreError> {
    if let Some(row) = store.find_by_conversation(conversation.agent, &conversation.id)? {
        if conversation.last_activity_at > row.last_activity_at {
            store.advance_activity(&row.id, conversation.last_activity_at)?;
            return Ok(Merged::Advanced(row.id));
        }
        return Ok(Merged::Unchanged);
    }
    let row = SessionRow {
        id: uuid::Uuid::new_v4().to_string(),
        agent: conversation.agent,
        cwd: conversation.cwd.clone(),
        name: history_name(store, conversation)?,
        agent_session_id: Some(conversation.id.clone()),
        status: Status::Inactive,
        needs_reason: None,
        created_at: conversation.started_at,
        last_activity_at: conversation.last_activity_at,
        ended_at: Some(conversation.last_activity_at),
        origin: Origin::History,
        sidebar_added_at: None,
        group_id: None,
        owner_named: false,
        interrupted_at: None,
    };
    store.insert(&row)?;
    Ok(Merged::Added(row.id))
}

/// `<agent>-<first 8 characters of the conversation id>`, longer if that is taken.
fn history_name(store: &Store, conversation: &Conversation) -> Result<String, StoreError> {
    let id: String = conversation
        .id
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect();
    for length in [8, 12, 16, id.len()] {
        let name = format!(
            "{}-{}",
            conversation.agent.id(),
            &id[..length.min(id.len())]
        );
        if !store.name_taken(&name)? {
            return Ok(name);
        }
    }
    for n in 2.. {
        let name = format!("{}-{id}-{n}", conversation.agent.id());
        if !store.name_taken(&name)? {
            return Ok(name);
        }
    }
    unreachable!("an unbounded counter always finds a free name")
}
