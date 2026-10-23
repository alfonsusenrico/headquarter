# Tasks

## 1. Refresh

- [x] 1.1 The host answers `RefreshUsage` once no usage read is under way, at most 40 s. Verify with `refresh_answers_after_the_new_reading` in `crates/hq-cli/tests/host.rs`, which failed before the change (42% instead of 77%) and passes after. Done (2026-10-23).
- [x] 1.2 The details block presses from the backdrop; Refresh shows "Refreshing…", disabled, with a spinner until answered. Verify with clippy and on screen. Done in code (2026-10-23).
- [ ] 1.3 On screen: open Codex's details, press Refresh; the details stay open, show "Refreshing…" with a spinner, then the new reading. Evidence: the owner's check.

Expected Result: refreshing usage keeps the details open and shows its progress.

## 2. Delivery

- [ ] 2.1 `./scripts/check` passes; write `verification.md`; push and open the PR.
