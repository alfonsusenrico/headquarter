## 1. Meter fill

- [x] 1.1 Fill and track share `METER_WIDTH`; unit test `the_fill_matches_the_percentage_of_the_track` (`cargo test -p hq-app`)
- [x] 1.2 `./scripts/check` passes on the branch (9c002ed, 510 tests, 2026-10-27)
- [x] 1.3 The owner sees a Claude Code session's meter at its true width in the real-case build. Accepted 2026-10-27 ("works").

Expected Result: the meter's fill matches the context used, checked by the unit test and on screen.

## 2. Delivery

- [ ] 2.1 After the owner accepts: write `verification.md`, archive the change, push the branch, open the pull request and report its URL
