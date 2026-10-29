# Tasks

## 1. Update log

- [x] 1.1 Top-aligned scrolling list holding every line (D1). Evidence: the dev app with a local update (`HQ_INSTALL_FROM` with a local release), on screen at the first lines and when full.
- [x] 1.2 Following the newest line unless scrolled back (D2). Evidence: unit test of the follow rule (at bottom, scrolled back, back at bottom), on screen scrolling back during a slowed local update.

Expected Result: the log reads from the top and follows the update.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes. `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `fix/update-log-scroll`, open the PR, report its URL.

Expected Result: one PR with the fix.
