# Proposal

## Why

The owner's primary bug in 0.1.2 (2026-10-23): dragging an image onto Headquarter does
nothing, while in Alacritty it types the image's path and Claude Code attaches it.
GPUI already reports file drops; the terminal pane did not listen for them.

## Expected Outcome

- Dropping one or more files or folders on the terminal pane types their paths into
  the agent, escaped as Terminal.app does and followed by a space; an image dropped on
  a Claude Code prompt becomes an attachment, as in Alacritty.

## What Changes

- `hq-app` (`terminal/keys.rs`): `dropped_paths` escapes shell characters with a
  backslash (letters outside ASCII are kept), separates paths with spaces, and ends
  with a space.
- `hq-app` (`terminal/view.rs`): the pane accepts dropped paths and sends that text as
  a paste (bracketed when the agent enabled it), and takes focus.

Decision (owner, 2026-10-23): plain, like Alacritty, with no highlight while dragging.

## Non-goals

- Dropping on the sidebar, an empty pane, or a session that is not attached.
- Pasting image data from the clipboard (agents read the clipboard themselves).

## Capabilities

### Modified Capabilities
- `terminal-pane`: files dropped on the pane are typed as their paths.

No project capability is added.

## Impact

`crates/hq-app/src/terminal/keys.rs`, `crates/hq-app/src/terminal/view.rs`. Checked
with Claude Code 2.1.309.

## Approval

Approved by the owner on 2026-10-23 ("Approve, plain").
