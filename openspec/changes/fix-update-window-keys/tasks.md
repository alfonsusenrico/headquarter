# Tasks

## 1. Fix

- [x] 1.1 Open and close through one place each, with focus taken and given back (D1). Evidence: build, clippy, the dev app on screen. Done 2026-10-28: `open_update_window` and `close_update_window` used by all five openings; clippy clean; on screen below.
- [x] 1.2 Escape in the phases with a dismiss button (D2). Evidence: unit test per phase, on screen: Check for Updates' notice closes on Escape, keys typed while it is open do not reach the stand-in session, and the terminal has the keyboard after it closes. Done 2026-10-28: `escape_closes_only_what_offers_a_way_out`; with a key-echoing stand-in, "x" before, nothing from "abc" or Escape while the notice was open, "y" and "z" after (`research/evidence/fix-update-window-keys/`).

Expected Result: Escape closes the update window, and keys never pass through it.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes. `verification.md` written. Done 2026-10-28: 542 tests, 0 failed.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `fix/update-notice-esc`, open the PR, report its URL.

Expected Result: one PR with the fix.
