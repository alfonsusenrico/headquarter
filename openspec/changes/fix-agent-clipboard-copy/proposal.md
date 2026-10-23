# Proposal

## Why

The owner could not copy from Headquarter's terminal (2026-10-23). Claude Code takes
the mouse, so a drag selects inside Claude Code, which then copies by asking the
terminal to set the clipboard (OSC 52). Alacritty does that by default; Headquarter's
terminal model received the request and dropped it.

## Expected Outcome

- Dragging over text selects it and copies it on release, the same way for every agent,
  whether or not the agent uses the mouse. A click without a drag, and the wheel, still
  reach an agent that uses the mouse. Cmd-C copies the selection too.
- Text copied by an agent through OSC 52 (for example Claude Code's own copy commands)
  lands on the system clipboard; agents still cannot read it.
- A "Copied" note confirms each copy; copies are tidied (no trailing spaces, shared
  margin or blank edges); selected text stays readable in themes with a light selection
  block; a drag keeps going outside the pane and scrolls back through history past the
  top or bottom (owner, on screen, 2026-10-23).

## What Changes

- `hq-app` (`terminal/view.rs`): Headquarter owns left-button drags: a press waits until
  the pointer reaches another cell (a drag: Headquarter selects) or comes up (a click:
  sent to an agent that uses the mouse, press and release together; otherwise it clears
  the selection). Releasing a drag, a double-click (word) or a triple-click (line)
  copies the selection. Before, a drag in an agent that uses the mouse sent only the
  press and the release, never the movement, so nothing could be selected or copied
  (found on screen 2026-10-23).

- `hq-app` (`terminal/model.rs`): the terminal's clipboard-store events are kept
  (alacritty_terminal's default `Osc52::OnlyCopy`: copy accepted, read refused).
- `hq-app` (`terminal/session.rs`): after each output chunk, kept copies go to the
  clipboard; on Linux the "selection" target goes to the primary selection, on macOS it
  is ignored as Alacritty does.

- `hq-app` (`terminal/copy.rs`): tidying and the note's words; (`terminal/view.rs`) the
  note, window-wide drag tracking and edge scrolling; (`terminal/frame.rs`, `theme.rs`)
  the selection text colour.
- `hq-config`: `colors.selection.text` is read (it was reported as unsupported).

## Non-goals

- Letting agents read the clipboard (OSC 52 paste).
- Copies from sessions not shown (they are not attached, so nothing arrives).

## Capabilities

### Modified Capabilities
- `terminal-pane`: drags select and copy for every agent; agent copies reach the
  clipboard.

No project capability is added.

## Impact

`crates/hq-app/src/terminal/model.rs`, `crates/hq-app/src/terminal/session.rs`,
`crates/hq-app/src/terminal/view.rs`. Agents' own drag features (such as Claude Code's
selection) are no longer used.
Checked with Claude Code 2.1.309. Attaching replays no output (the host asks the agent
to redraw), so an old copy cannot overwrite the clipboard when switching sessions.

Decisions (owner, 2026-10-23): Headquarter owns drags (rejected: passing every drag to
agents as Alacritty does; and a setting for both); then, once the log showed Claude
Code's full-screen view keeps its history itself, drags in full-screen agents that use
the mouse go to the agent (rejected: Headquarter selecting only the visible screen
there).

## Approval

Approved by the owner on 2026-10-23 ("Approve"; widened to drag selection with
auto-copy for every agent, "Headquarter owns drags"; then the note, tidying, readable
selection and edge scrolling, "Approve").
