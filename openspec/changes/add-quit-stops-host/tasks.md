# Tasks

## 1. App

- [x] 1.1 The dialog and its three choices, with the running count and the remote control line (D1, D3). Evidence: unit test of the body text (none, one, several running, remote on), the dev app on screen in both themes. Done 2026-10-27: `the_dialog_says_what_quit_stops`. Seen by the owner in the real app, light and dark not compared on screen.
- [x] 1.2 ⌘Q, the menu's Quit and the window's close button open it (D1, D4). Evidence: the dev app, each way. Done 2026-10-27 in the owner's real app.
- [x] 1.3 Quit stops the host, then the app, and a failure offers Quit Anyway (D2). Evidence: the dev app on an isolated host: after Quit no host runs, and the sessions resume on the next start. Done 2026-10-27 in the owner's real app: the host log shows `host stopping reason=stop requested`, `host stopped`, then the sessions resumed on the next start. Quit Anyway was not triggered.

Expected Result: quitting Headquarter is one action.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes. `verification.md` written.
- [x] 2.2 The owner checks it on the real app. Accepted 2026-10-27 ("works").
- [ ] 2.3 After the owner accepts: archive, push `feat/quit-stops-host`, open the PR, report its URL.
