# Design: move the cursor in the agent's box from the phone

## Context

The phone draws the terminal from the core's `alacritty_terminal`, which knows the cursor's row
and column and whether it is shown. The hidden typing field mirrors each edit relative to the
agent's cursor (Backspaces, then text), so edits stay right wherever that cursor is, as long as the
field starts afresh whenever the cursor moves.

## Decisions

### D1. ← and → in the key rows

A session's grid: six a row, `1 2 3 Esc ⇧Tab Tab` over `← ↑ ↓ → Ctrl-C` and the image key, ⏎
beside both rows. A shell's grid: four a row, `Esc Tab Ctrl-C Ctrl-D` over `← ↑ ↓ →`. The keys
send the arrows the agent's cursor mode expects (`ESC [ D` or `ESC O D`), as ↑ and ↓ do. While the
keyboard is open they fold into the sideways row as today.

Rejected: a third row of keys. It takes terminal rows from a phone screen for two keys.

### D2. A tap on the cursor's row moves the cursor there

A tap away from a link, on the row the terminal cursor is on (shown, view not scrolled back), sends
the arrow presses from the cursor to the tapped cell: one per character between them, a wide
character counting once, and no further right than the row's text (a tap past it goes to its
end). The typing field starts afresh, then the keyboard opens as before. A tap on another row only
opens the keyboard.

Rejected: moving across rows with ↑ and ↓. At the first line of its box Claude Code takes ↑ as
"previous message", so a tap could replace what the owner wrote. Rejected: the keyboard's
space-bar swipe. Keyboards report it as a selection change, which the mirror cannot tell from the
keyboard's own corrections; the owner chose keys and taps.

## Failure behaviour

- The agent hides its cursor, or draws its box elsewhere: a tap only opens the keyboard.
- Not connected: the arrows are dropped as typing is.

## Risks

- An agent whose cursor is not where its input is would get arrows it does not expect; ← and →
  move nothing harmful in Claude Code, Codex or a shell.
