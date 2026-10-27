# Verification: quitting Headquarter stops the host

Tested: branch `feat/quit-stops-host` @ 1919da2, from `main` @ 33f0a08. 2026-10-27.

## Commands

- `./scripts/check`: passed. Rust 510 tests, 0 failed, app and Android checks, OpenSpec 19 items,
  0 failed.
- Unit test `the_dialog_says_what_quit_stops`: the text for none, one and several running
  sessions, and the remote control line.

## Real app (owner, 2026-10-27)

A local release build of `main` with this change, `fix-claude-conversations` and
`fix-header-meter-fill` (`test/real-case` @ edc9f3f), installed with `scripts/install.sh`. The
owner quit with the dialog and reopened Headquarter twice. The host log shows each time
`host stopping reason=stop requested`, `host stopped`, then `host started` and the sessions
resumed (`resume=true`). The owner accepted it ("works").

## Not verified

- Quit Anyway: the host stopped each time, so the failure path was not shown. Its code path is
  the same Keep Running quit.
- Light and dark themes were not compared on screen. The dialog uses the theme's chrome tokens.
- Linux: not built or run (no local Linux build, as for every change).
