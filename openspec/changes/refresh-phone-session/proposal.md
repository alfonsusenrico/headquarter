# A cleaner session screen on the phone

Status: approved by the owner on 2026-10-27 ("merged 67, approve, go ahead").

## Why

The owner finds the phone's session screen dated (2026-10-27): "The input field and send button
feels old", and wants a cleaner terminal with more space. From the options page
(`research/prototypes/mobile-ui-options/index.html`) the owner chose D3 for the session pane and
terminal, "because it doesn't have container again it's cleaner and more space", and A for Home
(a separate, second change). Decisions from the owner on 2026-10-27: two PRs with the session
first, the keys fold to one row while the keyboard is open, the shell gets the same layout.

## Expected Outcome

- The terminal runs flush from edge to edge as today, with no card around it and no lost
  columns. Soft fades at its top and bottom edges show there is more above or below. When the
  owner has scrolled back, a "Live" button returns to the newest output.
- A slim header: a back icon, the title, and below it the status glyph and word, the agent and
  the folder. The shell opens from an icon button.
- The keys sit in a two-row grid, so none scroll out of reach: 1, 2, 3, Esc, ⇧Tab on the first
  row, ↑, ↓, Tab, Ctrl-C on the second, and a tall ⏎ in accent across both rows. While the
  keyboard is open the keys fold into one row that scrolls, with ⏎ pinned at its end.
- The text box is a rounded pill. A round Send button appears inside it only when there is
  text, replacing today's separate "Send" button. It grows to four lines, then scrolls.
- The shell has the same layout with its keys (Esc, Tab, ↑, ↓, Ctrl-C, Ctrl-D) in one row
  before ⏎.
- Behaviour is unchanged: Send types the text, then Enter. ⏎ sends Enter. The text box keeps
  its keyboard settings (the shell's command field still never autocorrects).

## What Changes

- `SessionScreen.kt`: header, key grid, folding row, composer.
- `TerminalView.kt`: edge fades and the Live button (`Screen.scrolledBack`, `phone.scroll`).
- Icons as vector drawables from Material Icons (Apache-2.0): back, shell, send, arrow down.
- `Common.kt` and `Theme.kt`: shared key and icon-button parts and any tokens they need.

## Non-goals

- Home, Pairing, Settings and Add sessions (the second change, direction A).
- The core, the protocol, the desktop app.
- New keys or gestures, such as swiping between sessions.

## Capabilities affected

`android-app`. No new project capability.

## Impact

Phone app UI only. Waits for PR #67 (row cache) to merge first, since both touch `TerminalView`.
