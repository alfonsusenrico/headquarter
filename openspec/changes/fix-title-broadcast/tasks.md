# Tasks

## 1. Titles

- [x] 1.1 Broadcast rows whose title appeared, changed, or went away in an import pass; make the history interval a host option (`HQ_HISTORY_INTERVAL_SECS`). Verify with `a_title_found_later_reaches_subscribers` in `crates/hq-cli/tests/host.rs`, which fails without the broadcast ("the new title is reported" after 10 s) and passes with it. Done (2026-10-22).
- [x] 1.2 On screen: restart the host, open the app, and open Add sessions; past conversations show titles. Evidence: the owner's check. Done: the owner confirmed it on the test build (`test/combined-build`), 2026-10-23.

Expected Result: past conversations show their titles, not generated names.

## 2. Delivery

- [ ] 2.1 `./scripts/check` passes; write `verification.md`; push and open the PR. Check passed (373 tests) and verification written (2026-10-22); push waits for the owner.
