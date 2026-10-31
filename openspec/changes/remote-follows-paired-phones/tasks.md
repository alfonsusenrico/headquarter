# Tasks

## 1. Build

- [x] 1.1 Host: gateway follows paired phones and shown codes (D1); `enabled` ignored (D2). Evidence: host tests (no phone: no endpoint; pair starts and keeps it; code expiry stops it; removing the last phone stops it and closes the connection), with `HQ_REMOTE_RELAYS=none`.
- [x] 1.2 CLI: `on` and `off` removed, `RemoteSet` answers the status (D3). Evidence: CLI and protocol tests.
- [ ] 1.3 Window: no switch, app step always open, reach line (D4). Evidence: on screen.
- [x] 1.4 Docs: settings reference and README mentions of `hq remote on`. Evidence: grep.

Expected Result: remote control runs exactly while a phone is paired.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes; stand-in phone pairs, is removed, and the endpoint stops (host log). On screen. `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `feat/remote-follows-phones`, open the PR, report its URL.

Expected Result: one PR.
