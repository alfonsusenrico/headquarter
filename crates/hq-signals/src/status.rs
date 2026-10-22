//! The five-state session status machine.
//!
//! The tracker keeps independent flags and derives the status with a fixed
//! precedence (needs-you > working > done-unread > done), so conflicting signals
//! resolve the same way regardless of arrival order. Work the agent runs in the
//! background counts as working. A session whose agent is not running is inactive
//! whatever its flags say; a running one never becomes inactive by waiting.

use serde::{Deserialize, Serialize};

use crate::event::{EventKind, NeedsReason};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    Working,
    NeedsYou,
    DoneUnread,
    Done,
    Inactive,
}

impl Status {
    /// The words the owner sees; the serialized names stay as they were stored.
    pub fn label(self) -> &'static str {
        match self {
            Status::Working => "working",
            Status::NeedsYou => "needs you",
            Status::DoneUnread => "idle, not read yet",
            Status::Done => "idle",
            // The agent is not running; opening the session resumes it.
            Status::Inactive => "idle",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Thresholds {
    /// For agents without structured events: output quiet for this long ends a turn.
    pub quiet_after_ms: u64,
    /// Output this soon after a keystroke is treated as the keystroke's echo.
    pub echo_window_ms: u64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Thresholds {
            quiet_after_ms: 5_000,
            echo_window_ms: 300,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SessionTracker {
    thresholds: Thresholds,
    /// Whether the agent reports turns through structured events. When it does not,
    /// output activity and quiet periods stand in for them.
    structured: bool,
    alive: bool,
    working: bool,
    needs: Option<NeedsReason>,
    unread: bool,
    /// Work the agent runs for the session outside its turn: Claude Code background
    /// shell commands, Codex sub-agents. Reported from outside the terminal stream.
    background: bool,
    /// Whether the owner has prompted or typed since launch. Agents report their own
    /// startup as a turn; nothing is unread until the owner has asked for something.
    engaged: bool,
    last_output_ms: u64,
    last_input_ms: Option<u64>,
}

impl SessionTracker {
    pub fn new(structured: bool, thresholds: Thresholds, now_ms: u64) -> Self {
        SessionTracker {
            thresholds,
            structured,
            alive: true,
            working: false,
            needs: None,
            unread: false,
            background: false,
            engaged: false,
            last_output_ms: now_ms,
            last_input_ms: None,
        }
    }

    pub fn needs_reason(&self) -> Option<NeedsReason> {
        self.needs
    }

    /// Whether the owner has prompted or typed since the agent started.
    pub fn engaged(&self) -> bool {
        self.engaged
    }

    pub fn apply(&mut self, event: EventKind, now_ms: u64) {
        // An agent that reports its turns (hooks or OSC 7501) is trusted over the
        // output-activity heuristic from then on.
        if matches!(
            event,
            EventKind::PromptSubmitted
                | EventKind::ToolActivity
                | EventKind::NeedsInput(_)
                | EventKind::TurnComplete
                | EventKind::TurnFailed
                | EventKind::Interrupted
        ) {
            self.structured = true;
        }
        match event {
            EventKind::Started => {
                self.alive = true;
                self.working = false;
                self.needs = None;
                self.unread = false;
            }
            EventKind::PromptSubmitted | EventKind::ToolActivity => {
                if event == EventKind::PromptSubmitted {
                    self.engaged = true;
                }
                self.working = true;
                self.needs = None;
                self.unread = false;
            }
            // A report without a reason must not hide one already known (Codex sends
            // the reason in a notification and a reasonless OSC 7501 report).
            EventKind::NeedsInput(NeedsReason::Unknown) if self.needs.is_some() => {}
            EventKind::NeedsInput(reason) => self.needs = Some(reason),
            // A finished turn also clears background work until the next report says it
            // is still running, so a turn without any shows done at once.
            EventKind::TurnComplete | EventKind::TurnFailed => {
                // Only a turn that was running leaves something unread; agents also
                // report completion when idle at startup or twice per turn.
                if self.engaged && (self.working || self.needs.is_some()) {
                    self.unread = true;
                }
                self.working = false;
                self.background = false;
                self.needs = None;
            }
            EventKind::Interrupted => {
                self.working = false;
                self.background = false;
                self.needs = None;
            }
            EventKind::Ended => {
                self.alive = false;
                self.working = false;
                self.background = false;
                self.needs = None;
            }
            EventKind::OutputActivity => {
                if self.structured {
                    return;
                }
                let is_echo = self.last_input_ms.is_some_and(|input| {
                    now_ms.saturating_sub(input) <= self.thresholds.echo_window_ms
                });
                if !is_echo && self.engaged {
                    self.working = true;
                    self.unread = false;
                    self.last_output_ms = now_ms;
                }
            }
            EventKind::Seen => {
                self.engaged = true;
                self.last_input_ms = Some(now_ms);
                if !self.working && self.needs.is_none() {
                    self.unread = false;
                }
            }
        }
    }

    /// The owner looked at the session without typing: what finished is now read,
    /// but looking is not asking for work, so it does not engage the session.
    pub fn viewed(&mut self) {
        if !self.working && self.needs.is_none() {
            self.unread = false;
        }
    }

    /// Whether the agent is running work for the session in the background.
    pub fn set_background(&mut self, busy: bool) {
        self.background = busy && self.alive;
    }

    /// Advances time-based transitions; call periodically.
    pub fn tick(&mut self, now_ms: u64) {
        if !self.structured
            && self.working
            && now_ms.saturating_sub(self.last_output_ms) >= self.thresholds.quiet_after_ms
        {
            self.working = false;
            self.unread = self.engaged;
        }
    }

    pub fn status(&self) -> Status {
        if !self.alive {
            Status::Inactive
        } else if self.needs.is_some() {
            Status::NeedsYou
        } else if self.working || self.background {
            Status::Working
        } else if self.unread {
            Status::DoneUnread
        } else {
            Status::Done
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A started session the owner has already typed into.
    fn tracker(structured: bool) -> SessionTracker {
        let mut tracker = SessionTracker::new(structured, Thresholds::default(), 0);
        tracker.apply(EventKind::Started, 0);
        tracker.apply(EventKind::Seen, 0);
        tracker
    }

    #[test]
    fn fresh_session_is_done() {
        assert_eq!(tracker(true).status(), Status::Done);
    }

    #[test]
    fn permission_request_while_working_needs_you() {
        let mut t = tracker(true);
        t.apply(EventKind::PromptSubmitted, 10);
        t.apply(EventKind::NeedsInput(NeedsReason::Permission), 20);
        assert_eq!(t.status(), Status::NeedsYou);
        assert_eq!(t.needs_reason(), Some(NeedsReason::Permission));
    }

    #[test]
    fn tool_activity_after_needs_you_is_working() {
        let mut t = tracker(true);
        t.apply(EventKind::PromptSubmitted, 10);
        t.apply(EventKind::NeedsInput(NeedsReason::Permission), 20);
        t.apply(EventKind::ToolActivity, 30);
        assert_eq!(t.status(), Status::Working);
    }

    #[test]
    fn new_prompt_after_needs_you_is_working() {
        let mut t = tracker(true);
        t.apply(EventKind::NeedsInput(NeedsReason::Question), 20);
        t.apply(EventKind::PromptSubmitted, 30);
        assert_eq!(t.status(), Status::Working);
    }

    #[test]
    fn needs_you_outranks_working_in_either_order() {
        let mut t = tracker(true);
        t.apply(EventKind::NeedsInput(NeedsReason::Permission), 10);
        t.apply(EventKind::OutputActivity, 11);
        assert_eq!(t.status(), Status::NeedsYou);
    }

    #[test]
    fn turn_complete_is_unread_until_seen() {
        let mut t = tracker(true);
        t.apply(EventKind::PromptSubmitted, 10);
        t.apply(EventKind::TurnComplete, 20);
        assert_eq!(t.status(), Status::DoneUnread);
        t.apply(EventKind::Seen, 30);
        assert_eq!(t.status(), Status::Done);
    }

    #[test]
    fn failed_turn_is_unread() {
        let mut t = tracker(true);
        t.apply(EventKind::PromptSubmitted, 10);
        t.apply(EventKind::TurnFailed, 20);
        assert_eq!(t.status(), Status::DoneUnread);
    }

    #[test]
    fn seen_while_needing_you_keeps_needs_you() {
        let mut t = tracker(true);
        t.apply(EventKind::NeedsInput(NeedsReason::Permission), 10);
        t.apply(EventKind::Seen, 20);
        assert_eq!(t.status(), Status::NeedsYou);
    }

    #[test]
    fn startup_turn_before_the_owner_engages_is_not_unread() {
        let mut t = SessionTracker::new(true, Thresholds::default(), 0);
        t.apply(EventKind::ToolActivity, 10);
        assert_eq!(t.status(), Status::Working);
        t.apply(EventKind::TurnComplete, 20);
        assert_eq!(t.status(), Status::Done);
    }

    #[test]
    fn unstructured_startup_output_is_not_work() {
        let mut t = SessionTracker::new(false, Thresholds::default(), 0);
        t.apply(EventKind::OutputActivity, 1_000);
        t.tick(10_000);
        assert_eq!(t.status(), Status::Done);
    }

    #[test]
    fn completion_without_a_running_turn_is_not_unread() {
        let mut t = tracker(true);
        t.apply(EventKind::TurnComplete, 10);
        assert_eq!(t.status(), Status::Done);
    }

    #[test]
    fn unknown_reason_keeps_a_known_one() {
        let mut t = tracker(true);
        t.apply(EventKind::NeedsInput(NeedsReason::Permission), 10);
        t.apply(EventKind::NeedsInput(NeedsReason::Unknown), 11);
        assert_eq!(t.needs_reason(), Some(NeedsReason::Permission));
        t.apply(EventKind::ToolActivity, 12);
        t.apply(EventKind::NeedsInput(NeedsReason::Unknown), 13);
        assert_eq!(t.needs_reason(), Some(NeedsReason::Unknown));
    }

    #[test]
    fn viewing_reads_without_engaging() {
        let mut t = SessionTracker::new(true, Thresholds::default(), 0);
        t.viewed();
        t.apply(EventKind::ToolActivity, 10);
        t.apply(EventKind::TurnComplete, 20);
        assert_eq!(t.status(), Status::Done, "startup turn is not unread");

        let mut t = tracker(true);
        t.apply(EventKind::PromptSubmitted, 10);
        t.apply(EventKind::TurnComplete, 20);
        t.viewed();
        assert_eq!(t.status(), Status::Done);
    }

    #[test]
    fn interrupt_returns_to_done_without_unread() {
        let mut t = tracker(true);
        t.apply(EventKind::PromptSubmitted, 10);
        t.apply(EventKind::Interrupted, 20);
        assert_eq!(t.status(), Status::Done);
    }

    #[test]
    fn agent_exit_is_inactive() {
        let mut t = tracker(true);
        t.apply(EventKind::PromptSubmitted, 10);
        t.apply(EventKind::Ended, 20);
        assert_eq!(t.status(), Status::Inactive);
    }

    #[test]
    fn a_running_session_stays_done_however_long_it_waits() {
        let mut t = tracker(true);
        t.tick(12 * 60 * 60 * 1000);
        assert_eq!(t.status(), Status::Done);
    }

    #[test]
    fn background_work_is_working_and_keeps_unread() {
        let mut t = tracker(true);
        t.apply(EventKind::PromptSubmitted, 10);
        t.apply(EventKind::TurnComplete, 20);
        t.set_background(true);
        assert_eq!(t.status(), Status::Working);
        t.apply(EventKind::NeedsInput(NeedsReason::Permission), 30);
        assert_eq!(t.status(), Status::NeedsYou, "needs you still comes first");
        t.apply(EventKind::ToolActivity, 40);
        t.apply(EventKind::TurnComplete, 50);
        assert_eq!(
            t.status(),
            Status::DoneUnread,
            "a finished turn clears it until reported again"
        );
        t.set_background(true);
        assert_eq!(t.status(), Status::Working);
        t.set_background(false);
        assert_eq!(t.status(), Status::DoneUnread);
        t.set_background(true);
        t.apply(EventKind::Ended, 60);
        assert_eq!(t.status(), Status::Inactive);
        t.set_background(true);
        assert_eq!(t.status(), Status::Inactive, "an ended agent runs nothing");
    }

    #[test]
    fn structured_agents_ignore_output_activity() {
        let mut t = tracker(true);
        t.apply(EventKind::OutputActivity, 1_000);
        assert_eq!(t.status(), Status::Done);
    }

    #[test]
    fn unstructured_output_then_quiet_completes_turn() {
        let mut t = tracker(false);
        t.apply(EventKind::OutputActivity, 1_000);
        t.tick(2_000);
        assert_eq!(t.status(), Status::Working);
        t.tick(6_000);
        assert_eq!(t.status(), Status::DoneUnread);
    }

    #[test]
    fn structured_signal_overrides_the_quiet_rule() {
        let mut t = tracker(false);
        t.apply(EventKind::OutputActivity, 1_000);
        t.apply(EventKind::ToolActivity, 1_100);
        t.tick(60_000);
        assert_eq!(t.status(), Status::Working);
        t.apply(EventKind::OutputActivity, 61_000);
        t.apply(EventKind::TurnComplete, 62_000);
        t.tick(120_000);
        assert_eq!(t.status(), Status::DoneUnread);
    }

    #[test]
    fn unstructured_keystroke_echo_is_not_work() {
        let mut t = tracker(false);
        t.apply(EventKind::Seen, 1_000);
        t.apply(EventKind::OutputActivity, 1_100);
        assert_eq!(t.status(), Status::Done);
        t.apply(EventKind::OutputActivity, 1_500);
        assert_eq!(t.status(), Status::Working);
    }
}
