# Verification

Working tree on `fix/terminal-in-use-elsewhere` after a50d5fe, 2026-10-28, macOS.

## Automated

- `./scripts/check`: exit 0. 545 Rust tests passed, 0 failed (1 ignored, as before); rustfmt,
  clippy `-D warnings`, cargo-deny; OpenSpec 19 of 19 valid (strict); Android ktlint, Lint
  (warnings as errors), unit tests, debug and unsigned release builds.
- New or changed tests:
  - `hq-cli` `a_paired_phone_follows_types_and_is_held_to_the_allow_list`: through the remote
    gateway, a subscriber receives the session naming "Test Phone" when the phone takes it and
    naming no phone when the desktop takes it back; `List` names it too.
  - `hq-app` `the_page_names_the_client_in_use_elsewhere`: the page's heading and note per
    holder, for a session and a shell.
  - `hq-app` `a_session_a_phone_has_names_the_phone`: the sidebar row carries the phone and keeps
    its status word.

## On screen

The synthetic host under `/private/tmp/hq-inuse` (relays on), the stand-in `claude`, the dev app,
the stand-in phone on the `hq-mobile` core ("Test Phone"), and the realme RMX1921 dev phone with
the debug APK from the checked tree, paired through n0's relay. Do Not Disturb and stay-awake on
for the phone check with the owner's go, restored afterwards.

Desktop:
- The phone opens the session the desktop shows: the pane shows the agent's mark, "Open on Test
  Phone", "The agent is drawing for the phone's screen." and "Use here", and no terminal; the
  sidebar row shows the phone mark in place of its age. The shell panel, still the desktop's,
  keeps its terminal.
- Letters typed on the page did not reach the agent. Switching to Finder and back left the
  session on the phone.
- Enter on the page took it back: the terminal returned at 69 columns with the keyboard, the row
  showed its age again, and the Enter did not reach the agent.
- "Use here" took it back.
- The phone opened the session's shell: the shell panel showed the page, "The shell is drawing
  for the phone's screen."; the agent pane stayed a terminal and the row had no phone mark.
- The app restarted while the phone had the session: it did not reopen it; the pane showed the
  same page, and "Use here" opened it and took it back.
- Opening the shown session again from its sidebar row took it back. A click on the selected
  row's title renames it, as before.

Phone (realme):
- Opening the session took it at once, with nothing to confirm; the desktop showed "Open on
  realme RMX1921" and the row's phone mark.
- "Use here" on the desktop: the phone returned home at once and showed "cash flow tracker dev is
  open on Alfonsuss-MacBook-Air".
- From the shell screen, the desktop taking the shell back sent the phone home as well, not to the
  session screen.

## Not verified

- Linux.
- TalkBack and VoiceOver on the page and the phone mark.
- A light theme on screen: the page uses the chrome tokens of the Details pane, as the inactive
  page does.
