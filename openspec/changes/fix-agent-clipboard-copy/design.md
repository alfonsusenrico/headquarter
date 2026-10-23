# Design

## Decisions

### D1. Headquarter owns drags

A left-button press waits: reaching another cell makes it a drag, which Headquarter
selects for every agent; coming up first makes it a click, sent (press and release
together) to an agent that enabled mouse reporting, unless Shift is held, and otherwise
it clears the selection. A double or triple click selects a word or a line at once.
Releasing a selection copies it. Rejected: passing drags to agents as Alacritty does
(agents without mouse support, such as agy, could not select; owner's choice), and a
setting for both.

### D2. Tidy copies

A copy removes trailing spaces from each line, the margin all lines share, and blank
lines at both ends; blank lines inside stay. A selection that started mid-row has no
margin on its first line, so that line does not count towards the shared margin. This
matches how Claude Code tidies its own copies. Rejected: re-joining lines an agent
wrapped itself (the terminal cannot tell them from real line breaks).

### D3. Drags follow the pointer everywhere

While a press from the pane is held, the window reports the pointer and the release,
so a drag keeps going outside the pane. Past the top or bottom edge, the view scrolls
through the scrollback every 50 ms, one line plus one per line height further out, at
most ten. A full-screen agent has no scrollback; its selection covers what is on
screen, as in Alacritty.

### D4. Said, and readable

Each copy shows "Copied" or "Copied N lines" at the bottom right of the pane for 1.5 s.
Selected text takes the theme's selection text colour when it has one
(`colors.selection.text`, now read from settings and theme files; Alacritty's
`CellForeground` keeps cell colours), as Catppuccin sets for a light selection block.

### D5. Agent copies

Text an agent asks the terminal to copy (OSC 52) goes to the clipboard; alacritty_terminal's
default `Osc52::OnlyCopy` refuses requests to read it.
