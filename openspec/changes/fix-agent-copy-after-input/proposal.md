# Proposal

## Why

The owner found that pasting gave the wrong text after switching sessions
(2026-10-24). A diagnostic build, logging only lengths and fingerprints, showed why:
every time a session was opened, Claude Code redrew and sent its last selection again as
an OSC 52 copy, and Headquarter (since 0.1.4) put every agent copy on the clipboard. The
clipboard snapped back to the opened session's old selection, with a "Copied" note.

## Expected Outcome

- An agent's copy reaches the clipboard only right after the owner's input to that
  session: a drag's release, a click, or a key (Claude Code's copy-on-select, `/copy`).
- A copy an agent sends on its own, such as on a redraw when a session is opened, is
  ignored: the clipboard is unchanged and no note shows.

## What Changes

- `hq-app` (`terminal/session.rs`): the session records when the owner last sent it
  input (keys, a paste, a mouse report); an agent copy more than 2 seconds after that is
  dropped. Terminal replies Headquarter sends itself do not count as input.

## Non-goals

- Headquarter's own selection and copy are unchanged.
- Agents' highlighted selections stay as the agent draws them.

## Capabilities

### Modified Capabilities
- `terminal-pane`: agent copies count only right after the owner's input.

No project capability is added.

## Impact

`crates/hq-app/src/terminal/session.rs`. Seen with Claude Code 2.1.309 in its full-screen
view.

## Approval

Approved by the owner on 2026-10-24 ("Approve").
