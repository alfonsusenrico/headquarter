# Tasks

## 1. App

- [ ] 1.1 The dialog and its three choices, with the running count and the remote control line (D1, D3). Evidence: unit test of the body text (none, one, several running, remote on), the dev app on screen in both themes.
- [ ] 1.2 ⌘Q, the menu's Quit and the window's close button open it (D1, D4). Evidence: the dev app, each way.
- [ ] 1.3 Quit stops the host, then the app, and a failure offers Quit Anyway (D2). Evidence: the dev app on an isolated host: after Quit no host runs, and the sessions resume on the next start.

Expected Result: quitting Headquarter is one action.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check` passes. `verification.md` written.
- [ ] 2.2 The owner checks it on the real app.
- [ ] 2.3 After the owner accepts: archive, push `feat/quit-stops-host`, open the PR, report its URL.
