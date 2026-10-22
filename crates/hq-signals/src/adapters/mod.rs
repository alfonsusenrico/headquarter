//! Per-agent launch instrumentation and signal interpretation.
//!
//! Every instrument is passed on the launched agent's command line. Agent
//! configuration files are only ever read (to keep the owner's own status line
//! working), never written.

pub mod agy;
pub mod claude;
pub mod codex;
pub mod hook;

use std::path::Path;

use crate::event::{Agent, EventKind, NeedsReason, Source};
use crate::scanner::{ProgramState, TerminalSignal};

/// Variables an agent sets for the commands it runs, marking them as part of its own
/// session. When Headquarter itself was started from inside an agent, the host
/// inherits them, and an agent it launches would act as that session's child: Claude
/// Code turns transcript saving off ("inherited CLAUDE_CODE_CHILD_SESSION marker"),
/// Codex believes it runs in a sandbox. They are removed from every launch; the
/// agents' own settings variables are kept. Names read from claude 2.1.304 (the
/// variables it sets for its shells and drops for its own background process) and
/// codex 0.163.0.
pub const PARENT_SESSION_MARKERS: &[&str] = &[
    "AI_AGENT",
    "CLAUDECODE",
    "CLAUDE_CODE_BRIDGE_SESSION_ID",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_CHROME_MCP_ORG_DENIED",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_EVAL_INTERVIEW_SESSION",
    "CLAUDE_CODE_EXECPATH",
    "CLAUDE_CODE_HOST_WORKTREE",
    "CLAUDE_CODE_HOST_WORKTREE_FENCE",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_CODE_SESSION_ATTENDED",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_EFFORT",
    "CLAUDE_PID",
    "CODEX_SANDBOX",
    "CODEX_SANDBOX_NETWORK_DISABLED",
    "CODEX_THREAD_ID",
    "CODEX_TOOL_CALL_ID",
];

/// Which conversation a launched agent opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conversation<'a> {
    /// A new conversation; `id` is used by agents that accept a chosen id (Claude).
    New { id: &'a str },
    /// Continue the agent's own conversation `id`, or open its picker when unknown.
    Resume(Option<&'a str>),
}

pub struct LaunchContext<'a> {
    /// Executable providing the `hook-forward` and `hook-statusline` subcommands.
    pub helper_exe: &'a Path,
    /// Socket that `hook-forward` delivers hook payloads to.
    pub hook_socket: &'a Path,
    pub conversation: Conversation<'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSpec {
    pub program: String,
    pub args: Vec<String>,
    /// Whether the agent reports turns through structured events.
    pub structured: bool,
}

/// The owner's own settings that launch instrumentation must keep working.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnerSettings {
    pub claude_status_line: Option<String>,
}

impl OwnerSettings {
    /// Reads the owner's settings for a session in `cwd`. Read-only.
    pub fn discover(cwd: &Path) -> OwnerSettings {
        OwnerSettings {
            claude_status_line: claude::owner_status_line(cwd),
        }
    }
}

pub fn launch_spec(agent: Agent, ctx: &LaunchContext<'_>, owner: &OwnerSettings) -> LaunchSpec {
    match agent {
        Agent::Claude => claude::launch_spec(ctx, owner),
        Agent::Codex => codex::launch_spec(ctx.conversation),
        Agent::Agy => agy::launch_spec(ctx.conversation),
    }
}

/// The keys that close an agent from its own interface, in order, as observed in each
/// agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseKeys {
    /// Stops a running turn; sent only while the session is working.
    pub interrupt: &'static [u8],
    /// Sent before clearing, to dismiss anything covering the prompt.
    pub dismiss: &'static [u8],
    /// Clears unsent text in the prompt without leaving the agent.
    pub clear: &'static [u8],
    /// Commands typed and entered one after another.
    pub exit: &'static [&'static str],
}

const ESCAPE: &[u8] = b"\x1b";
const CTRL_C: &[u8] = b"\x03";
const CTRL_U: &[u8] = b"\x15";

/// One Ctrl-C on an empty Codex prompt quits it and leaves its commands running, and
/// Ctrl-C does not clear agy's prompt, so both clear with Ctrl-U. Codex's `/stop` ends
/// the background commands its shared server would otherwise keep running.
pub fn close_keys(agent: Agent) -> CloseKeys {
    match agent {
        Agent::Claude => CloseKeys {
            interrupt: ESCAPE,
            dismiss: b"",
            clear: CTRL_C,
            exit: &["/exit"],
        },
        Agent::Codex => CloseKeys {
            interrupt: ESCAPE,
            dismiss: b"",
            clear: CTRL_U,
            exit: &["/stop", "/quit"],
        },
        Agent::Agy => CloseKeys {
            interrupt: ESCAPE,
            dismiss: ESCAPE,
            clear: CTRL_U,
            exit: &["/quit"],
        },
    }
}

/// Interprets a terminal signal for `agent`. Signal text is inspected here and
/// discarded; only the resulting event kind leaves this function.
pub fn terminal_event(agent: Agent, signal: &TerminalSignal) -> Option<(Source, EventKind)> {
    if let TerminalSignal::ProgramStatus(status) = signal {
        if status.id.is_some() {
            return None;
        }
        let event = match status.state {
            ProgramState::Working => EventKind::ToolActivity,
            ProgramState::Blocked => {
                EventKind::NeedsInput(status.kind.unwrap_or(NeedsReason::Unknown))
            }
            ProgramState::Done => EventKind::TurnComplete,
            ProgramState::Error => EventKind::TurnFailed,
            // Codex reports the end of every turn as idle rather than done.
            ProgramState::Idle if agent == Agent::Codex => EventKind::TurnComplete,
            ProgramState::Idle => EventKind::Interrupted,
            ProgramState::Clear => return None,
        };
        return Some((Source::ProgramStatus, event));
    }
    match agent {
        Agent::Codex => codex::terminal_event(signal),
        Agent::Claude | Agent::Agy => None,
    }
}

/// Quotes `value` for a POSIX shell, for hook commands that agents run through `sh`.
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::ProgramStatus;

    #[test]
    fn shell_quote_handles_single_quotes() {
        assert_eq!(shell_quote("a b"), "'a b'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
    }

    #[test]
    fn program_status_maps_for_every_agent() {
        let blocked = TerminalSignal::ProgramStatus(ProgramStatus {
            state: ProgramState::Blocked,
            kind: Some(NeedsReason::Question),
            id: None,
        });
        for agent in Agent::ALL {
            assert_eq!(
                terminal_event(agent, &blocked),
                Some((
                    Source::ProgramStatus,
                    EventKind::NeedsInput(NeedsReason::Question)
                ))
            );
        }
    }

    #[test]
    fn child_program_status_records_are_ignored() {
        let child = TerminalSignal::ProgramStatus(ProgramStatus {
            state: ProgramState::Done,
            kind: None,
            id: Some("task".into()),
        });
        assert_eq!(terminal_event(Agent::Claude, &child), None);
    }
}
