# Tasks

## 1. Fix

- [x] 1.1 Draw the question last (D1). Evidence: the dev app on screen with Remote Control, Check for Updates and Usage open in turn, ⌘Q showing the question above each. Done 2026-10-28: above Remote Control, the Usage window and the update notice (`research/evidence/fix-quit-over-windows/`).
- [x] 1.2 Give focus back on Cancel (D2). Evidence: on screen, Esc cancels the question, then Esc closes Remote Control. Done 2026-10-28: the second Esc closed Remote Control and the Usage window.

Expected Result: Quit's question is always seen.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes. `verification.md` written. Done 2026-10-28: 541 tests, 0 failed.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `fix/quit-over-remote`, open the PR, report its URL.

Expected Result: one PR with the fix.
