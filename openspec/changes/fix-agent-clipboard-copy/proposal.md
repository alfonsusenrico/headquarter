# Proposal

## Why

The owner could not copy from Headquarter's terminal (2026-10-23). Claude Code takes
the mouse, so a drag selects inside Claude Code, which then copies by asking the
terminal to set the clipboard (OSC 52). Alacritty does that by default; Headquarter's
terminal model received the request and dropped it.

## Expected Outcome

- Text copied by an agent through OSC 52, such as a mouse selection in Claude Code,
  lands on the system clipboard.
- Agents still cannot read the clipboard.

## What Changes

- `hq-app` (`terminal/model.rs`): the terminal's clipboard-store events are kept
  (alacritty_terminal's default `Osc52::OnlyCopy`: copy accepted, read refused).
- `hq-app` (`terminal/session.rs`): after each output chunk, kept copies go to the
  clipboard; on Linux the "selection" target goes to the primary selection, on macOS it
  is ignored as Alacritty does.

## Non-goals

- Letting agents read the clipboard (OSC 52 paste).
- Copies from sessions not shown (they are not attached, so nothing arrives).

## Capabilities

### Modified Capabilities
- `terminal-pane`: agent copies reach the clipboard.

No project capability is added.

## Impact

`crates/hq-app/src/terminal/model.rs`, `crates/hq-app/src/terminal/session.rs`.
Checked with Claude Code 2.1.309. Attaching replays no output (the host asks the agent
to redraw), so an old copy cannot overwrite the clipboard when switching sessions.

## Approval

Approved by the owner on 2026-10-23 ("Approve").
