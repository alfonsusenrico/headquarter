# Tasks

## 1. Host

- [ ] 1.1 `Hosted` keeps the Claude conversation id. Hook and status-line payloads from another conversation are ignored, and `SessionStart` `clear` or an unknown id is adopted (D1, D2). Evidence: host integration tests with a stub agent sending hooks under one `HQ_SESSION` with its own and a foreign conversation id. The foreign "waiting" event leaves the status working, a `clear` switches the id, the foreign status line does not change the details.
- [ ] 1.2 The rate-limited log line (D3). Evidence: unit test of the limiter.
- [ ] 1.3 On the owner's machine: run the dev host with a real Claude Code session and confirm no foreign events change it. Evidence: the log line count and the status over a working turn.

Expected Result: a hidden Claude Code session can no longer mark another session as needing the owner.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check` passes. `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `fix/hook-session-match`, open the PR, report its URL.

Expected Result: one PR with the fix.
