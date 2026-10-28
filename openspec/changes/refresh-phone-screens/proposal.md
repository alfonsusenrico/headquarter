# The phone's other screens in direction A

Status: proposed 2026-10-28, waiting for the owner's approval.

## Why

The owner chose direction A ("Refined desktop") for the phone's screens other than the session
screen (2026-10-27, options page `research/prototypes/mobile-ui-options/index.html`). The
session screen shipped in D3 (#70). Home, Pairing, Settings and Add sessions still use text
characters for icons (⚙, ‹, ⌄, ›, "+"), a wrapping usage line and the first release's shapes.
On 2026-10-28 the owner asked for all four screens in one change, with the large title shrinking
on scroll and the Add button shrinking to an icon while scrolling.

## Expected Outcome

- **Home:** a large "Headquarter" title that folds away as the list scrolls and returns at the
  top. Under it, a full-width row for the computer: its connection dot, name and state
  ("connected", "reconnecting…"), a chevron that opens the switcher, and Settings. Usage shows
  as equal tiles, one per agent, each with the window, the percentage and a bar. Needs-you and
  unread cards, group headers and rows keep their content with rounder shapes and real icons.
  "working" is the accent word with stepping dots, as on the desktop since 0.1.15. "+ Add" is a
  floating button that shrinks to a round + while the list scrolls down.
- **Computers sheet:** the same rows, with a plus icon on "Pair another computer".
- **Pairing:** the same flow and wording, laid out as three numbered steps above the camera,
  which gets corner marks instead of a coloured border. "Enter the code instead" is a full-width
  button.
- **Settings and Add sessions:** the same sections, rows and wording, with an icon back button,
  rounder cards, pill buttons, and a search icon in Add sessions' field.
- Behaviour is unchanged: what each screen lists, folding groups, switching computers, pairing,
  adding sessions, and the connection messages.

## What Changes

- `ui/Common.kt`: the shared header (`Crumb`), card, button, section label and new icon parts.
- `ui/HomeScreen.kt`: the collapsing title, the computer row, usage tiles, cards and rows, the
  working dots, the floating Add button, the computers sheet.
- `ui/PairScreen.kt`, `ui/SettingsScreen.kt`, `ui/AddScreen.kt`: the shared parts and their own
  layout changes above.
- `res/drawable`: settings, add, search, chevron down and chevron right icons from Material Icons
  (Apache-2.0), beside the four added in #70.

## Non-goals

- The session and shell screens (D3, shipped in #70).
- The core (`hq-mobile`), the protocol, the desktop app.
- New features: no new settings, actions, gestures or navigation (no bottom bar).
- The app lock screen.

## Capabilities affected

`android-app`. No new project capability.

## Impact

Phone app UI only. No agent CLI is involved: checked with the stand-in agent on a test host and
the dev phone.
