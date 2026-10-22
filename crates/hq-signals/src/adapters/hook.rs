//! Normalizes hook and notify payloads into metadata-only event records.
//!
//! Claude Code and Codex share hook event names and the `session_id`, `cwd`, and
//! `hook_event_name` fields. Only those fields and `tool_name` are copied; prompt,
//! assistant message, and tool input values are never read into the record.

use serde_json::Value;

use crate::event::{Agent, EventKind, EventRecord, NeedsReason, Source};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookError {
    InvalidJson,
    MissingSessionId,
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            HookError::InvalidJson => "payload is not a JSON object",
            HookError::MissingSessionId => "payload has no session id",
        })
    }
}

/// Returns `Ok(None)` for valid payloads that carry no status meaning.
pub fn normalize(agent: Agent, raw: &str, now_ms: u64) -> Result<Option<EventRecord>, HookError> {
    let value: Value = serde_json::from_str(raw).map_err(|_| HookError::InvalidJson)?;
    value.as_object().ok_or(HookError::InvalidJson)?;

    let session_id = string_field(&value, "session_id").ok_or(HookError::MissingSessionId)?;
    let Some(event) = hook_event(&value) else {
        return Ok(None);
    };
    let mut record = EventRecord::new(now_ms, agent, Source::Hook, event);
    record.session_id = Some(session_id);
    record.cwd = string_field(&value, "cwd");
    if matches!(
        event,
        EventKind::ToolActivity | EventKind::NeedsInput(NeedsReason::Permission)
    ) {
        record.tool_name = string_field(&value, "tool_name");
    }
    Ok(Some(record))
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_owned)
}

fn hook_event(value: &Value) -> Option<EventKind> {
    let name = value.get("hook_event_name").and_then(Value::as_str)?;
    match name {
        "SessionStart" => Some(EventKind::Started),
        "UserPromptSubmit" => Some(EventKind::PromptSubmitted),
        "PreToolUse" | "PostToolUse" => Some(EventKind::ToolActivity),
        "PermissionRequest" => Some(EventKind::NeedsInput(NeedsReason::Permission)),
        "Notification" => notification_event(value),
        "Stop" => Some(EventKind::TurnComplete),
        "StopFailure" => Some(EventKind::TurnFailed),
        "Interrupt" => Some(EventKind::Interrupted),
        "SessionEnd" => Some(EventKind::Ended),
        _ => None,
    }
}

/// Claude Code `Notification` hooks. `idle_prompt` and `agent_completed` repeat
/// what `Stop` already reported, so they carry no new status.
fn notification_event(value: &Value) -> Option<EventKind> {
    match value.get("notification_type").and_then(Value::as_str)? {
        "permission_prompt" => Some(EventKind::NeedsInput(NeedsReason::Permission)),
        "elicitation_dialog" => Some(EventKind::NeedsInput(NeedsReason::Question)),
        "agent_needs_input" => Some(EventKind::NeedsInput(NeedsReason::Input)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &str = "SECRET-TEXT-42";

    fn claude(raw: &str) -> Result<Option<EventRecord>, HookError> {
        normalize(Agent::Claude, raw, 7)
    }

    fn serialized(record: &EventRecord) -> String {
        serde_json::to_string(record).unwrap()
    }

    #[test]
    fn prompt_text_never_reaches_the_record() {
        let raw = format!(
            r#"{{"session_id":"s1","cwd":"/w","hook_event_name":"UserPromptSubmit","prompt":"{SECRET}","transcript_path":"/t.jsonl"}}"#
        );
        let record = claude(&raw).unwrap().unwrap();
        assert_eq!(record.event, EventKind::PromptSubmitted);
        assert_eq!(record.session_id.as_deref(), Some("s1"));
        assert!(!serialized(&record).contains(SECRET));
        assert!(!serialized(&record).contains("transcript"));
    }

    #[test]
    fn assistant_message_and_tool_input_are_dropped() {
        let stop = format!(
            r#"{{"session_id":"s1","hook_event_name":"Stop","last_assistant_message":"{SECRET}"}}"#
        );
        let record = claude(&stop).unwrap().unwrap();
        assert_eq!(record.event, EventKind::TurnComplete);
        assert!(!serialized(&record).contains(SECRET));

        let permission = format!(
            r#"{{"session_id":"s1","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{{"command":"{SECRET}","description":"{SECRET}"}}}}"#
        );
        let record = claude(&permission).unwrap().unwrap();
        assert_eq!(record.event, EventKind::NeedsInput(NeedsReason::Permission));
        assert_eq!(record.tool_name.as_deref(), Some("Bash"));
        assert!(!serialized(&record).contains(SECRET));
    }

    #[test]
    fn notification_types_map_to_reasons() {
        let make = |kind: &str| {
            format!(
                r#"{{"session_id":"s","hook_event_name":"Notification","notification_type":"{kind}","message":"{SECRET}"}}"#
            )
        };
        let event = |kind: &str| claude(&make(kind)).unwrap().map(|record| record.event);
        assert_eq!(
            event("permission_prompt"),
            Some(EventKind::NeedsInput(NeedsReason::Permission))
        );
        assert_eq!(
            event("elicitation_dialog"),
            Some(EventKind::NeedsInput(NeedsReason::Question))
        );
        assert_eq!(
            event("agent_needs_input"),
            Some(EventKind::NeedsInput(NeedsReason::Input))
        );
        assert_eq!(event("idle_prompt"), None);
    }

    #[test]
    fn lifecycle_events_map() {
        let event = |name: &str| {
            claude(&format!(
                r#"{{"session_id":"s","hook_event_name":"{name}"}}"#
            ))
            .unwrap()
            .map(|record| record.event)
        };
        assert_eq!(event("SessionStart"), Some(EventKind::Started));
        assert_eq!(event("PostToolUse"), Some(EventKind::ToolActivity));
        assert_eq!(event("StopFailure"), Some(EventKind::TurnFailed));
        assert_eq!(event("SessionEnd"), Some(EventKind::Ended));
        assert_eq!(event("SubagentStop"), None);
        assert_eq!(event("SomethingNew"), None);
    }

    #[test]
    fn malformed_payloads_are_rejected() {
        assert_eq!(claude("not json"), Err(HookError::InvalidJson));
        assert_eq!(claude("[1,2]"), Err(HookError::InvalidJson));
        assert_eq!(
            claude(r#"{"hook_event_name":"Stop"}"#),
            Err(HookError::MissingSessionId)
        );
    }

    #[test]
    fn codex_hooks_share_claude_names() {
        let raw = r#"{"session_id":"t1","hook_event_name":"Interrupt","turn_id":"u","permission_mode":"default"}"#;
        let record = normalize(Agent::Codex, raw, 9).unwrap().unwrap();
        assert_eq!(record.event, EventKind::Interrupted);
    }
}
