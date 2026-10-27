//! Antigravity CLI: it reads hooks and its status line only from its own settings
//! files and has no per-launch override, so nothing is injected. It reports working,
//! blocked (auth, permission), and done itself with OSC 7501 (verified with agy
//! 1.3.6 and 1.4.7); output activity is the fallback until the first such report.

use super::{Conversation, LaunchSpec};
use crate::event::NeedsReason;
use crate::scanner::ProgramStatus;

/// agy 1.4.7 ends a turn that leaves an artifact to review with `blocked`, kind permission,
/// and the message "Approve plan", though its prompt is ready and the review is optional
/// (fix-agy-artifact-review). The message is base64 and compared as sent.
const ARTIFACT_REVIEW: &str = "QXBwcm92ZSBwbGFu";

/// Whether `status` is agy's artifact review rather than a wait for the owner.
pub fn is_artifact_review(status: &ProgramStatus) -> bool {
    status.kind == Some(NeedsReason::Permission)
        && status
            .message
            .as_ref()
            .is_some_and(|message| message.0 == ARTIFACT_REVIEW)
}

/// agy has no conversation picker, so resuming without a known id starts a new
/// conversation in the folder.
pub fn launch_spec(conversation: Conversation<'_>) -> LaunchSpec {
    let args = match conversation {
        Conversation::Resume(Some(id)) => vec!["--conversation".to_owned(), id.to_owned()],
        Conversation::Resume(None) | Conversation::New { .. } => Vec::new(),
    };
    LaunchSpec {
        program: "agy".to_owned(),
        args,
        structured: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launches_without_instrumentation() {
        let spec = launch_spec(Conversation::New { id: "unused" });
        assert_eq!(spec.program, "agy");
        assert!(spec.args.is_empty());
        assert!(!spec.structured);
    }

    #[test]
    fn resume_known_conversation() {
        let spec = launch_spec(Conversation::Resume(Some("c1")));
        assert_eq!(spec.args, ["--conversation", "c1"]);
        assert!(launch_spec(Conversation::Resume(None)).args.is_empty());
    }
}
