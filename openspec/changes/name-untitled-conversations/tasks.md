# Tasks

## 1. Untitled conversations

- [x] 1.1 `untitled_id` and `display_title`; the muted id in Add sessions. Verify with `a_past_conversation_without_a_title_is_untitled_with_its_id` (`hq-protocol`) and the Add sessions tests. Done (2026-10-23).
- [x] 1.2 On screen: untitled past conversations read "Untitled conversation · <id>" in Add sessions. Evidence: the owner's check. Done: the owner confirmed it on the test build (`test/combined-build`), 2026-10-23.

Expected Result: no past conversation shows only a generated name.

## 2. Delivery

- [ ] 2.1 `./scripts/check` passes; write `verification.md`; push and open the PR after offer-continuable-sessions. Check passed (383 tests) and verification written (2026-10-23).
