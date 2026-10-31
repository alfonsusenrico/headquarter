# Replies that are not the end

Status: approved by the owner 2026-10-31 ("ok let's do it", with fix-claude-usage-folder and fix-phone-scroll).

## Why

The owner (2026-10-29): agents reply but keep working, through background work or by carrying on
at once. Headquarter notifies "replied" each time, which reads as "that session's work is done",
and the update's Wait restarted at such a moment and cut a session.

Evidence (`research/evidence/false-done/findings.md`): over 4 days, 484 of the owner's Claude Code
turns were followed by a turn Claude started itself when background work reported back (54 within
5 s, 68 within 10 s). The "replied" notice and the updater's idle test both key on the turn's end
alone; 0.1.20 sees background work ("background") but neither uses it; the updater restarts in the
same instant (0.4 s after the turn end on 2026-10-28).

## Expected Outcome

- No "replied" notification while a session's background work runs, or when the agent carries on
  within 10 seconds. It comes once the session has stayed at its prompt, with no background work,
  for 10 seconds.
- On the phone, a reply whose background work still runs stays in its group showing "background"
  instead of rising to the top as "Claude replied".
- Wait in the update window also waits for sessions with background work and for plans to review.
  It still restarts as soon as nothing is busy (owner: no delay).

## What Changes

- Desktop notifications: "replied" waits for the session to settle (D1, D2).
- Phone home: replies with background work stay out of the top section (D3).
- Updater: background work and plans to review count as busy (D4).
- Specs: `notifications`, `android-app`, `app-update`.

## Non-goals

- The sidebar's status and unread mark: a finished turn still shows as unread at once.
- Needs-you notifications: they stay immediate.
- A settle time for the update restart (owner chose none).
- Work Headquarter cannot see (a command that leaves its process tree); the 10 s quiet time covers
  quick returns only.

## Capabilities

No new project capability. Modified: `notifications`, `android-app`, `app-update`.

## Impact

`hq-app` (`notifications.rs`, `workspace.rs`, `update.rs`), `hq-mobile` (`mirror.rs`). No protocol
or host change. Verified against Claude Code 2.1.312 transcripts (shape only) and the stand-ins.
