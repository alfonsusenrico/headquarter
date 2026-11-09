# Design: type into the agent's own box on the phone, and tap links

## Context

`SessionScreen.kt` shows the terminal, the key row and `Composer`, a text box whose Send calls
`Phone::type_text`: the text, then `\r` 80 ms later (`ENTER_DELAY`, so Claude Code takes it as
typing, not a paste). The terminal is a Canvas with pinch (text size) and drag (scroll). The core
keeps an `alacritty_terminal::Term` per open terminal, fed by the host's output, so it holds the
cells' OSC 8 links. The desktop finds a link at a cell in `hq-app/src/terminal/links.rs` (OSC 8
and plain `http(s)` addresses across wrapped rows, web addresses only).

Observed 2026-11-06 without submitting anything (`research/evidence/phone-typing-images/`):
Claude Code 2.1.318 and Codex 0.166.0 both insert a new line on Ctrl+J (`\n`) at their prompt.

## Goals

- One place to type: the agent's own box. The keyboard's own features keep working.
- Several lines without sending; sending stays one tap.
- Links open from the phone, safely (web addresses only, as on the desktop).

## Decisions

### D1. A hidden field mirrors its edits into the session

A text field the owner never sees holds the keyboard's input. Tapping the terminal focuses it,
which opens the keyboard. On each change the app compares the field's old and new text: it erases
what changed at the end (one Backspace, `\x7f`, per character, counted in grapheme clusters as the
agents' boxes delete) and types the new text. A word Gboard corrects is erased and retyped; a
composing word shows in the agent's box as it is composed. The cursor is kept at the end of the
field, so the keyboard's cursor gestures do nothing.

Rejected: raw key events with the field's input type set to none (as Termux does). Simpler and
exact, but Gboard then offers no autocorrect, swipe or voice typing (owner chose the mirror,
2026-11-06). Rejected: keeping a text box and adding tap-to-focus. It keeps two boxes for one
message, which is the problem.

### D2. The field empties whenever the agent's box may have changed

The field holds only what was typed since the last send or key. The key row's keys (⏎, Esc,
↑, ↓, Tab, 1-3, Ctrl-C) empty it first, since each can change or clear the agent's box. Backspace
on an empty field still erases in the agent's box (text recalled with ↑, for example): the field
keeps one invisible placeholder character, and deleting it sends a Backspace and restores it.
Which of a key event and the placeholder Gboard uses on an empty field is checked on the phone
before choosing; the placeholder is the fallback that works with any keyboard.

Rejected: reading the agent's box back from the screen to stay in step. Each agent draws its box
differently, and the screen is redrawn after the edit arrives, so it would race the typing.

### D3. The keyboard's Enter is a new line for an agent and Enter for a shell

In an agent session the field is multi-line: the keyboard's Enter inserts `\n`, which the mirror
sends as Ctrl+J, a new line in Claude Code and Codex. The key row's ⏎ sends `\r` and empties the
field. In a shell the field is single-line with the keyboard's action "Go": it sends `\r` and
empties the field, and it keeps today's shell settings (no autocorrect, no capitals).

For agy the new-line key is unverified. Until it is checked, an agy session uses the shell's rule
(Enter sends), so a message is never sent half-written by accident in an unknown way.

### D4. The core answers which link is at a cell

`Phone::link_at(row, col) -> Option<String>`: the web address at that cell of the screen as drawn
(scrolled back or not), from the same finder the desktop uses. A tap on a link opens it with the
system browser (Android `ACTION_VIEW`) and does not open the keyboard. A long press on a link copies
it and shows "Link copied". A tap elsewhere opens the keyboard (D1); a drag still scrolls and a
pinch still sizes the text. Only `http` and `https` open, as on the desktop.

Rejected: sending every link's cells with each screen snapshot so the app could underline them.
It finds links on every frame for a hint the owner did not ask for; recorded as a non-goal.

### D5. The link finder moves to its own crate

`hq-app/src/terminal/links.rs` depends only on `alacritty_terminal`. It moves to a new crate
`hq-links` (same `alacritty_terminal` pin), with its tests; `hq-app` and `hq-mobile` both use it.

Rejected: a copy in `hq-mobile`. Two finders would drift (trailing punctuation, wrapped rows).
Rejected: putting it in `hq-signals`, which has no terminal dependency and would gain one for
every crate that uses it.

### D6. `type_text` becomes `edit`

`Phone::edit(erase: u32, text: String)` sends `erase` Backspaces then `text` (with `\n` kept), in
one input write, with no Enter after it, and returns the view to live output as typing does today.
`send_key(Enter)` sends. `type_text` and `ENTER_DELAY` go: an Enter now always comes from a
separate tap.

## Failure behaviour

- Not connected: edits are dropped as typing is today, and the "Reconnecting…" banner shows. The
  field empties when the link comes back, since the agent's box may have changed.
- A link that is not a web address: the tap opens the keyboard, as a tap on text.
- No browser on the phone: a short message, "No app can open this link".

## Risks

- Keyboards differ in how they replace a word (whole word or a few characters). The mirror
  handles any replacement at the end; a change in the middle of the field (moving the cursor) is
  prevented by pinning the cursor.
- Typing over a slow relay shows each letter late in the agent's box. Today's box hides that
  until Send. Accepted by the owner's choice.

## Open questions

- agy's new-line key (D3), once agy trusts a folder.
