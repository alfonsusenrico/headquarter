# Design: scrolling full-screen agents on the phone

## Context

`Phone::scroll(lines)` calls `Terminal::scroll`, which moves alacritty's display offset. The core
terminal tracks the agent's modes (`TermMode`) and already encodes keys for them. The desktop's
`TerminalView::scroll` chooses between wheel reports, arrow keys and its own history.

## Decisions

### D1. The desktop's three paths, in the core

`Terminal::scroll(lines, row, col)` returns the bytes to send, or moves the history itself:

- The agent asked for SGR mouse reports (`MOUSE_MODE` with `SGR_MOUSE`, as Claude Code does): one
  wheel report per line, button 64 to go back (drag down) and 65 forward, at `row`/`col`
  (`ESC[<64;col;rowM`, clamped to the grid), at most 10 per call, as the desktop. The older
  encoding is not sent, as on the desktop.
- Alternate screen otherwise: Up or Down arrows, in the application-cursor form when that mode is
  set, at most 10.
- Otherwise: its own history, as today.

`Phone::scroll` sends returned bytes as input, which makes the phone the device in use as typing
does; it does not return the view to the latest output.

Rejected: encoding in Kotlin (the modes live in the core, and both phone apps to come would repeat
it). Rejected: sharing the desktop's code (it lives in `hq-app` with GPUI types; the encoding is a
few lines, tested on both sides).

### D2. Where the drag is

The drag's centroid gives the cell (`row`, `col`) from the same cell size the view draws with,
clamped to the grid. Claude Code scrolls the pane under that cell.

## Risks

- One line per report can feel slow in long conversations; the desktop's per-event cap of 10
  keeps fast drags bounded. Tune on the device.
