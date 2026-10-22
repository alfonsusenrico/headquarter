//! Normalized agent events and the metadata-only record that may be persisted.
//!
//! `EventRecord` deliberately has no field that can hold prompt text, model output,
//! or tool input; normalizers copy only the fields listed here.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Agent {
    Claude,
    Codex,
    Agy,
}

impl Agent {
    pub const ALL: [Agent; 3] = [Agent::Claude, Agent::Codex, Agent::Agy];

    pub fn id(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
            Agent::Agy => "agy",
        }
    }

    /// Executable name looked up on `PATH`.
    pub fn binary(self) -> &'static str {
        self.id()
    }

    pub fn from_id(id: &str) -> Option<Agent> {
        Agent::ALL.into_iter().find(|agent| agent.id() == id)
    }
}

impl std::fmt::Display for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.pad(self.id())
    }
}

/// Why a session needs the owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NeedsReason {
    Permission,
    Question,
    Auth,
    Input,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "reason", rename_all = "kebab-case")]
pub enum EventKind {
    Started,
    PromptSubmitted,
    ToolActivity,
    NeedsInput(NeedsReason),
    TurnComplete,
    TurnFailed,
    /// The owner cancelled the turn; they are present, so nothing is left unread.
    Interrupted,
    Ended,
    /// Terminal output from an agent that has no structured working signal.
    OutputActivity,
    /// The owner interacted with the session.
    Seen,
}

/// Where an event came from, kept so evidence can show which signal worked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Source {
    Hook,
    Notify,
    ProgramStatus,
    Notification,
    Title,
    Output,
    Input,
    Process,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventRecord {
    pub at_ms: u64,
    pub agent: Agent,
    pub source: Source,
    pub event: EventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

impl EventRecord {
    pub fn new(at_ms: u64, agent: Agent, source: Source, event: EventKind) -> Self {
        EventRecord {
            at_ms,
            agent,
            source,
            event,
            session_id: None,
            cwd: None,
            tool_name: None,
        }
    }
}
