# Tasks

## 1. Feed

- [x] 1.1 Subscribe before listing and keep changes that arrive before an answer. Verify with `a_change_while_the_feed_opens_is_not_lost` in `hq-client`, which fails with the old order ("expected the link, got None") and passes with the fix. Done (2026-10-22).
- [x] 1.2 On screen: with idle Claude sessions in other terminals, open the app and Add sessions; each shows as in another terminal at once. Evidence: the owner's check. Done: the owner confirmed it on the test build (`test/combined-build`), 2026-10-23.

Expected Result: idle sessions in other terminals are not offered for adding.

## 2. Delivery

- [ ] 2.1 `./scripts/check` passes; write `verification.md`; push and open the PR. Check passed (373 tests) and verification written (2026-10-22); push waits for the owner.
