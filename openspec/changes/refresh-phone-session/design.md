# Design: a cleaner session screen on the phone

## Context

Today (`SessionScreen.kt`): a `Crumb` header with a "›_ Shell" text button, the terminal, one key
row that scrolls sideways with ⏎ pinned, and a bordered text field beside a 72 dp "Send" button.
Chosen design: D3 on the options page (`research/prototypes/mobile-ui-options/index.html`),
with the owner's changes of 2026-10-27 (keys fold while typing, shell in the same layout).

## Goals and non-goals

Goals: more terminal, fewer containers, every key in reach, a composer that looks current, the
same behaviour. Non-goals: Home and the other screens (second change), core or protocol work.

## Decisions

### D1. The terminal stays flush, with fades and Live

No card or margins around the terminal, so the columns stay as today (8 dp padding a side). A
16 dp gradient from the terminal's background to transparent at the top edge, and at the bottom
edge while scrolled back, drawn over the terminal. While `Screen.scrolledBack > 0` a small tonal
"Live" button with a down arrow floats at the bottom right and calls
`phone.scroll(-scrolledBack)`.

Rejected: a card or outlined pane (D2) costs a column and is the container the owner wants gone.

### D2. A slim header

56 dp: a back icon button, then the title (16 sp, semibold, one line) over a line with the
status glyph, the status word in its status colour, the agent and the folder (13 sp, muted, one
line, cut at the end). A tonal icon button opens the shell. The shell's own header says
"Shell · title" and the folder, without the shell button.

### D3. The key grid, folding while typing

Keyboard closed: a grid of 5 equal columns and 2 rows of keys (48 dp tall, 6 dp gaps), and a 6th
column holding ⏎, as tall as both rows, filled with the accent colour. The session's keys: row 1 holds
1, 2, 3, Esc and ⇧Tab, row 2 holds ↑, ↓, Tab and Ctrl-C, and its fifth slot stays empty. The shell's six keys
(Esc, Tab, ↑, ↓, Ctrl-C, Ctrl-D) share one row in 6 equal columns, then ⏎ one row tall, so the
shell's keys take one row.

A key's label shrinks to fit, down to 10 sp, since the shell's seven keys share a narrow row
(found on the dev phone: "Ctrl-C" was cut off at 13 sp).

Keyboard open (`WindowInsets.ime` visible): the keys become one row that scrolls sideways, with
⏎ pinned at its end, as today, so the terminal keeps about one more row.

Rejected: one scrolling row always (today: keys out of reach), an accessory bar only (D1/D2's
morphing Enter-or-Send button, which changes meaning under the owner's thumb).

### D4. The pill composer

The text box is a pill (fully rounded, tonal surface, no border), 48 dp tall for one line, growing
to four lines, then scrolling. When it holds text, a 36 dp round Send button in accent appears at
its right edge inside the pill. The keyboard's Send action does the same. Placeholders as today.
Keyboard options as today (the shell: no autocorrect, URI keyboard).

### D5. Icons as vector drawables

Back, shell (terminal prompt), send and arrow down come from Material Icons (Apache-2.0) as
vector drawables in `res/drawable`, drawn with `Icon(painterResource(...))`. Rejected: the
`material-icons-extended` library, a large dependency for four icons.

## Accessibility

Every icon button has a content description (Back, Open the shell, Send, Back to live output).
Keys keep 48 dp height. The status keeps its glyph and word, never colour alone. Contrast of the
accent ⏎ label and the Send icon on accent is checked in both themes.

## Risks

- The grid costs about two terminal rows against today's single row while the keyboard is
  closed. Accepted by the owner for keys in reach. It folds while typing.
- Folding on `ime` visibility animates the layout. The switch happens once the keyboard is shown,
  not per frame of its animation.
