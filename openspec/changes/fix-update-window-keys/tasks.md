# Tasks

## 1. Fix

- [ ] 1.1 Open and close through one place each, with focus taken and given back (D1). Evidence: build, clippy, the dev app on screen.
- [ ] 1.2 Escape in the phases with a dismiss button (D2). Evidence: unit test per phase, on screen: Check for Updates' notice closes on Escape, keys typed while it is open do not reach the stand-in session, and the terminal has the keyboard after it closes.

Expected Result: Escape closes the update window, and keys never pass through it.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check` passes. `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `fix/update-notice-esc`, open the PR, report its URL.

Expected Result: one PR with the fix.
