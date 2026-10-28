# Design: the phone's other screens in direction A

## Context

The phone app (`mobile/android`, Jetpack Compose, Material 3) draws its own parts over the
desktop's tokens (`ui/Theme.kt`, add-android-app D11). Home (`HomeScreen.kt`) has a one-line
header with the title, a computer chip and a "⚙" character, a usage line that wraps, cards for
needs-you and unread, folding groups, and a "+ Add" pill. Pairing, Settings and Add sessions
share `Crumb` (a "‹" character), `Card` (14 dp corners), `HqButton` (12 dp corners) and
`SectionLabel`.

Direction A on the options page (`research/prototypes/mobile-ui-options/index.html`) mocks Home,
Pairing and the computers sheet. Settings and Add sessions have no mock: they take the same
parts. Owner decisions of 2026-10-28: all four screens in one change, the large title shrinks
on scroll, and Add shrinks to an icon while scrolling.

## Goals and non-goals

Goals: the four screens read as one finished app with the desktop's look, real icons, and the
same behaviour. Non-goals: the session screen (#70), the core, new features or navigation.

## Decisions

### D1. Shared parts

- **Header (`Crumb`):** a 56 dp bar with a 48 dp back icon button in the accent colour (vector
  `ic_back`), the title at 17 sp semibold and its optional subtitle at 12.5 sp muted.
- **Card:** 16 dp corners, the card surface and its 1 dp line, as today otherwise.
- **Button (`HqButton`):** a pill (fully rounded), 48 dp tall. Primary on accent, others on the
  card surface with the line. The equal-width rule for button groups stays.
- **Section label:** 12 sp, semibold, upper case, muted, as the mock.
- **Icons:** Material Icons (Apache-2.0) as vector drawables: settings, add, search, chevron
  down, chevron right, beside back, terminal, send and arrow down from #70. Every icon button
  keeps a content description.

### D2. Home's header: a title that folds away, and the computer row

At the top of the list, the title "Headquarter" at 26 sp in a 56 dp row. Under it, the computer
row, full width and 56 dp: the connection dot, the computer's name (one line, cut at the end),
its state in muted text, a chevron down, then the settings icon button. Tapping the row (not the
settings icon) opens the computers sheet.

Scrolling the list down folds the title row away, following the scroll
(`TopAppBarDefaults.exitUntilCollapsedScrollBehavior`), and it unfolds once the list is back at
the top. The computer row stays. So the header is 112 dp at rest and 56 dp scrolled, against
today's single 56 dp row.

The settings icon sits in the computer row rather than the title row (the mock puts it beside
the title), so it stays in reach when the title has folded away.

The state word follows the link: "connected", "connecting…", "reconnecting…", "not connected".
The dot keeps today's colours and its breathing while not connected. The row's description for
TalkBack: "Computer: <name>, <state>. Switch computers".

Rejected: Material's `LargeTopAppBar` (152 dp tall at rest, far more than the mock), and keeping
the title and the computer in one row as today (long computer names get cut).

### D3. Usage tiles

One tile per meter, in a row of equal widths with 8 dp gaps: the agent's name (12.5 sp
semibold), then "window · N%" (12 sp muted), then a 5 dp bar, red from 80% as today. One or two
meters take the full width between them. More than three wrap into rows of three. Tiles use the
card surface, line and 14 dp corners.

### D4. Cards and rows

- **Needs-you and unread cards:** as today (4 dp edge, glyph, title, detail · age) with 16 dp
  corners, at least 68 dp tall, and a chevron right icon.
- **Group header:** as today with a chevron icon (down when open, right when folded) instead of
  ⌄ and ›.
- **Session row:** at least 52 dp. Content as today.
- **"working":** the word in the accent colour followed by three dots that light one after
  another, then all rest for a step, four steps a second, as the desktop does
  (refresh-pane-and-status D1). "starting" uses the same dots in muted text. The breathing fade
  goes.

### D5. The floating Add button

Material's `ExtendedFloatingActionButton` with the add icon and "Add", 56 dp tall, 16 dp
corners, accent, at the bottom right above the navigation bar. It is expanded at the top of the
list and while scrolling up, and shrinks to the round icon while scrolling down. Its
description stays "Add" when shrunk. It shows only while connected or reconnecting, as today.

### D6. Computers sheet

As today (radio, name, "Shown now" or "Seen 2h ago"), with 16 dp corners on the card and the
"Pair another computer" action as a full-width row with the add icon in accent.

### D7. Pairing

Same flow, checks and wording. Laid out as the mock: the heading "Pair with your computer"
(22 sp), three numbered steps (accent numbers on the accent-soft circle):

1. Open Headquarter on your computer.
2. Choose Headquarter ▸ Remote Control… ▸ Pair a Phone.
3. Point the camera at the code. While typing: Enter the code shown under the QR code.

The camera fills the width in a square with 20 dp corners and four white corner marks, instead
of the accent border. "Enter the code instead" is a full-width pill button. The code field, Pair,
the error and "The code works once and expires in 2 minutes." stay as today.

### D8. Settings and Add sessions

- **Settings:** sections, rows, switches, radios and wording as today, in the new cards. "Pair
  another computer" gets the add icon. Unpair becomes a small pill outlined in the line colour
  with the danger text.
- **Add sessions:** the search field becomes a pill with the search icon at its start. The
  folders' cards, rows and wording stay. Each row's Add becomes a small pill: accent outline and
  text, then "Added" muted.

## Accessibility

- Text and icon contrast is checked in both themes against the tokens: muted text on the card
  and the page, accent on the page, the label on the accent button, the amber detail on its card.
  The options page measured them for A (lowest 5.7:1 for text, 3.9:1 for the amber glyph).
- Touch targets stay at least 48 dp. The folding title does not take Settings or the switcher
  out of reach.
- Status keeps its glyph and word. The dots are decoration beside the word "working".

## Risks

- The header at rest is 56 dp taller than today, one session row less until the list scrolls.
- `exitUntilCollapsedScrollBehavior` belongs to Material 3's experimental top app bar API and
  needs an opt-in. A future Material update could change it. The app pins Material 3 through
  its Compose BOM.
