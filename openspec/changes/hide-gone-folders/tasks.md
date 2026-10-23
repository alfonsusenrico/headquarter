# Tasks

## 1. Add sessions

- [x] 1.1 Leave out conversations whose folder is gone, on open and on each update. Verify with `conversations_whose_folder_is_gone_are_not_offered` (`hq-app`). Done (2026-10-23).
- [x] 1.2 On screen: Add sessions no longer lists `af-eval-*` or `/tmp/hqreal` conversations, and Antigravity's count drops. Evidence: the owner's check. Done: the owner confirmed it on the test build (`test/combined-build`), 2026-10-23.

Expected Result: Add sessions offers only conversations that can be continued.

## 2. Delivery

- [ ] 2.1 `./scripts/check` passes; write `verification.md`; push and open the PR after mark-external-sessions-unaddable. Check passed (376 tests) and verification written (2026-10-23).
