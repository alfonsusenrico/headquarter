//! Codex: reports working, blocked, and idle itself with OSC 7501 (verified with
//! codex 0.163.0), so it is launched exactly as the owner would start it.
//!
//! Not injected:
//! - configuration overrides (`-c`): since 0.163.0 any override makes Codex leave its
//!   shared background server and warn about it on every start (owner report,
//!   2026-10-20); the OSC 9 notifications they once enabled only added the reason to
//!   a blocked report, which OSC 7501 already carries.
//! - `notify`: it fired "turn complete" while a turn was still running, and leaving
//!   it alone keeps the owner's own notify program untouched.
//! - hooks: hooks passed with `-c` are held for review on every launch and never run
//!   until trusted.

use super::{Conversation, LaunchSpec};
use crate::event::{EventKind, NeedsReason, Source};
use crate::scanner::TerminalSignal;

pub fn launch_spec(conversation: Conversation<'_>) -> LaunchSpec {
    let mut args = Vec::new();
    if let Conversation::Resume(id) = conversation {
        args.push("resume".to_owned());
        args.extend(id.map(str::to_owned));
    }
    LaunchSpec {
        program: "codex".to_owned(),
        args,
        structured: true,
    }
}

/// OSC 9 notifications, which the owner's own Codex settings may turn on.
pub fn terminal_event(signal: &TerminalSignal) -> Option<(Source, EventKind)> {
    let TerminalSignal::Notification { body, .. } = signal else {
        return None;
    };
    let body = body.to_lowercase();
    (body.starts_with("approval requested") || body.contains("wants to")).then_some((
        Source::Notification,
        EventKind::NeedsInput(NeedsReason::Permission),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resume_uses_the_resume_subcommand() {
        let known = launch_spec(Conversation::Resume(Some("t1")));
        assert_eq!(known.args[known.args.len() - 2..], ["resume", "t1"]);
        let picker = launch_spec(Conversation::Resume(None));
        assert_eq!(picker.args.last().map(String::as_str), Some("resume"));
    }

    #[test]
    fn launched_without_configuration_overrides() {
        let spec = launch_spec(Conversation::New { id: "unused" });
        assert!(spec.args.is_empty(), "{:?}", spec.args);
    }

    #[test]
    fn approval_notification_needs_you() {
        let signal = TerminalSignal::Notification {
            title: None,
            body: "Approval requested: rm -rf build".into(),
        };
        assert_eq!(
            terminal_event(&signal),
            Some((
                Source::Notification,
                EventKind::NeedsInput(NeedsReason::Permission)
            ))
        );
    }

    #[test]
    fn titles_are_not_status() {
        assert_eq!(
            terminal_event(&TerminalSignal::Title("\u{280b} terminal".into())),
            None
        );
    }
}
