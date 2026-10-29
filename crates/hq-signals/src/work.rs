//! Work an agent keeps running after its turn, and plans it left for review
//! (add-background-work-status D6, D7). Read from what the agent reports and the processes its
//! background commands started; nothing is guessed from the rest of its process tree.

use std::collections::{HashMap, HashSet};

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::adapters::agy;
use crate::event::Agent;
use crate::scanner::{ProgramState, ProgramStatus};

/// What still runs in the background, as clients show it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackgroundWork {
    /// Background agents working now.
    #[serde(default)]
    pub agents: u16,
    /// Background shell commands still running.
    #[serde(default)]
    pub commands: u16,
    /// Monitors still watching a command.
    #[serde(default)]
    pub monitors: u16,
    /// The newest working agent's task, as the agent titled it. Display only: never
    /// written to the index or the log.
    #[serde(default)]
    pub agent_title: Option<String>,
}

impl BackgroundWork {
    pub fn is_empty(&self) -> bool {
        self.agents == 0 && self.commands == 0 && self.monitors == 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Command,
    Monitor,
}

/// One session's background work and review flag.
#[derive(Debug, Default)]
pub struct WorkTracker {
    /// Working background agents by record id, oldest first, with their titles.
    agents: Vec<(String, Option<String>)>,
    /// Background calls started but not answered yet: the processes that ran before.
    pending: HashMap<String, (Kind, HashSet<u32>)>,
    /// Background commands and monitors with the processes they started.
    running: Vec<(Kind, Vec<u32>)>,
    /// Sub-agents found outside the terminal (Codex rollouts).
    subagents: u16,
    review: bool,
}

impl WorkTracker {
    /// Takes in a status report. Returns whether anything shown changed.
    pub fn program_status(&mut self, agent: Agent, status: &ProgramStatus) -> bool {
        let before = (self.summary(), self.review);
        match &status.id {
            // Claude Code reports each background agent as a child record.
            Some(id) => match status.state {
                ProgramState::Working => {
                    let title = status.title.as_ref().and_then(|title| decode(&title.0));
                    match self.agents.iter_mut().find(|(known, _)| known == id) {
                        Some(entry) => entry.1 = title.or(entry.1.take()),
                        None => self.agents.push((id.clone(), title)),
                    }
                }
                _ => self.agents.retain(|(known, _)| known != id),
            },
            None => {
                // A plan to review lasts until the agent reports anything else.
                self.review = agent == Agent::Agy && agy::is_artifact_review(status);
                // Clearing the root record clears every record (Program Status 0.4).
                if status.state == ProgramState::Clear {
                    self.agents.clear();
                }
            }
        }
        before != (self.summary(), self.review)
    }

    /// Takes in a Claude Code hook payload; `children` lists the agent's processes now and
    /// is called only for background calls. Returns whether anything shown changed.
    pub fn hook(&mut self, raw: &Value, children: impl FnOnce() -> HashSet<u32>) -> bool {
        let Some(kind) = background_kind(raw) else {
            return false;
        };
        let Some(call) = raw.get("tool_use_id").and_then(Value::as_str) else {
            return false;
        };
        match raw.get("hook_event_name").and_then(Value::as_str) {
            Some("PreToolUse") => {
                self.pending.insert(call.to_owned(), (kind, children()));
                false
            }
            Some("PostToolUse") => {
                let Some((kind, before)) = self.pending.remove(call) else {
                    return false;
                };
                let started: Vec<u32> = children().difference(&before).copied().collect();
                if started.is_empty() {
                    return false;
                }
                self.running.push((kind, started));
                true
            }
            Some("PostToolUseFailure") => {
                self.pending.remove(call);
                false
            }
            _ => false,
        }
    }

    /// Drops commands whose processes have all exited. Returns whether anything changed.
    pub fn prune(&mut self, alive: impl Fn(u32) -> bool) -> bool {
        let before = self.running.len();
        self.running
            .retain(|(_, pids)| pids.iter().any(|pid| alive(*pid)));
        before != self.running.len()
    }

    /// Sub-agents found from the agent's own files (Codex). Returns whether it changed.
    pub fn set_subagents(&mut self, count: u16) -> bool {
        let changed = self.subagents != count;
        self.subagents = count;
        changed
    }

    /// Whether any command is still followed, so the host knows to keep checking.
    pub fn follows_processes(&self) -> bool {
        !self.running.is_empty()
    }

    /// Every process followed for a background command.
    pub fn followed_processes(&self) -> Vec<u32> {
        self.running
            .iter()
            .flat_map(|(_, pids)| pids.iter().copied())
            .collect()
    }

    pub fn summary(&self) -> BackgroundWork {
        let count = |kind| {
            let total = self
                .running
                .iter()
                .filter(|(each, _)| *each == kind)
                .count();
            u16::try_from(total).unwrap_or(u16::MAX)
        };
        BackgroundWork {
            agents: u16::try_from(self.agents.len())
                .unwrap_or(u16::MAX)
                .saturating_add(self.subagents),
            commands: count(Kind::Command),
            monitors: count(Kind::Monitor),
            agent_title: self
                .agents
                .iter()
                .rev()
                .find_map(|(_, title)| title.clone()),
        }
    }

    pub fn review(&self) -> bool {
        self.review
    }
}

/// A background Bash call or a Monitor call; anything else is not followed.
fn background_kind(raw: &Value) -> Option<Kind> {
    match raw.get("tool_name").and_then(Value::as_str)? {
        "Monitor" => Some(Kind::Monitor),
        "Bash" => raw
            .get("tool_input")
            .and_then(|input| input.get("run_in_background"))
            .and_then(Value::as_bool)
            .filter(|background| *background)
            .map(|_| Kind::Command),
        _ => None,
    }
}

/// A title as Claude Code sends it: base64 text, cut to one short line.
fn decode(title: &str) -> Option<String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(title.trim())
        .ok()?;
    let text = String::from_utf8(bytes).ok()?;
    let line: String = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let short: String = line.chars().take(80).collect();
    (!short.is_empty()).then_some(short)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::scanner::Message;

    fn report(state: ProgramState, id: Option<&str>, title: Option<&str>) -> ProgramStatus {
        ProgramStatus {
            state,
            kind: None,
            id: id.map(str::to_owned),
            message: None,
            title: title.map(|title| Message(title.to_owned())),
        }
    }

    #[test]
    fn background_agents_follow_their_child_records() {
        // As Claude Code 2.1.312 sent them (evidence/add-background-work-status/claude-bg-agent).
        let mut work = WorkTracker::default();
        let title = "QmFja2dyb3VuZCBzbGVlcCB0YXNr"; // "Background sleep task"
        assert!(work.program_status(
            Agent::Claude,
            &report(ProgramState::Working, Some("a8d59"), Some(title))
        ));
        assert!(!work.program_status(Agent::Claude, &report(ProgramState::Done, None, None)));
        let summary = work.summary();
        assert_eq!(summary.agents, 1);
        assert_eq!(
            summary.agent_title.as_deref(),
            Some("Background sleep task")
        );
        assert!(work.program_status(
            Agent::Claude,
            &report(ProgramState::Clear, Some("a8d59"), None)
        ));
        assert!(work.summary().is_empty());
    }

    #[test]
    fn a_cleared_root_clears_every_agent() {
        let mut work = WorkTracker::default();
        work.program_status(
            Agent::Claude,
            &report(ProgramState::Working, Some("a"), None),
        );
        work.program_status(
            Agent::Claude,
            &report(ProgramState::Working, Some("b"), None),
        );
        assert_eq!(work.summary().agents, 2);
        work.program_status(Agent::Claude, &report(ProgramState::Clear, None, None));
        assert_eq!(work.summary().agents, 0);
    }

    #[test]
    fn background_commands_are_their_processes() {
        let mut work = WorkTracker::default();
        let pre = json!({"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_use_id": "t1",
            "tool_input": {"command": "x", "run_in_background": true}});
        let post = json!({"hook_event_name": "PostToolUse", "tool_name": "Bash", "tool_use_id": "t1",
            "tool_input": {"command": "x", "run_in_background": true}});
        // An MCP server (10) runs throughout; the call starts a shell (20) and its child (21).
        assert!(!work.hook(&pre, || HashSet::from([10])));
        assert!(work.hook(&post, || HashSet::from([10, 20, 21])));
        assert_eq!(work.summary().commands, 1);
        assert!(work.follows_processes());
        assert!(
            !work.prune(|pid| pid == 21),
            "one of its processes still runs"
        );
        assert!(work.prune(|pid| pid == 10), "only the MCP server is left");
        assert!(work.summary().is_empty());
    }

    #[test]
    fn foreground_calls_and_other_tools_are_not_followed() {
        let mut work = WorkTracker::default();
        let mut asked = false;
        let plain = json!({"hook_event_name": "PreToolUse", "tool_name": "Bash", "tool_use_id": "t",
            "tool_input": {"command": "x"}});
        assert!(!work.hook(&plain, || {
            asked = true;
            HashSet::new()
        }));
        assert!(!asked, "no process listing for a foreground call");
        let monitor_pre =
            json!({"hook_event_name": "PreToolUse", "tool_name": "Monitor", "tool_use_id": "m"});
        let monitor_post =
            json!({"hook_event_name": "PostToolUse", "tool_name": "Monitor", "tool_use_id": "m"});
        work.hook(&monitor_pre, HashSet::new);
        assert!(work.hook(&monitor_post, || HashSet::from([30])));
        assert_eq!(work.summary().monitors, 1);
    }

    #[test]
    fn an_agy_review_lasts_until_the_next_report() {
        let mut work = WorkTracker::default();
        let review = ProgramStatus {
            state: ProgramState::Blocked,
            kind: Some(crate::event::NeedsReason::Permission),
            id: None,
            message: Some(Message("QXBwcm92ZSBwbGFu".into())), // "Approve plan"
            title: None,
        };
        assert!(work.program_status(Agent::Agy, &review));
        assert!(work.review());
        assert!(work.program_status(Agent::Agy, &report(ProgramState::Working, None, None)));
        assert!(!work.review(), "approved, or a new prompt");
        assert!(
            !WorkTracker::default().program_status(Agent::Claude, &review),
            "agy only"
        );
    }
}
