# Tasks

## 1. Reading agy

- [x] 1.1 `read_agy` and `parse_agy_usage` in `hq-signals`. Verify with `agy_usage_gives_each_pool_its_windows`, `a_failed_or_empty_agy_answer_has_no_usage`, and `an_agent_without_a_usage_source_gets_one_plain_line`. Done (2026-10-23).
- [x] 1.2 The host reads agy on refresh, every 10 minutes while usage shows, and after a hosted agy turn; keeps `quota/agy.json`; refresh waits for it. Verify with `agy_usage_comes_from_its_own_usage_command` (refresh, pools, the read runs in `quota/agy-workspace`). Done (2026-10-23).
- [x] 1.3 Usage tests stub `agy` so they never start the machine's own. Verify: the host tests pass and leave no `/tmp/hqh-*` folder. Done (2026-10-23).

Expected Result: agy's usage is read like Claude's and Codex's.

## 2. Showing it

- [x] 2.1 `ring_windows`: the rings and their tooltip follow the pool nearest its limit. Verify with `agy_rings_follow_the_pool_nearest_its_limit`. Done (2026-10-23).
- [ ] 2.2 On screen: agy's ring filled, its tooltip and details naming the pools, Refresh. Evidence: the owner's check.

Expected Result: agy's ring and details show its pools.

## 3. Delivery

- [ ] 3.1 `./scripts/check` passes; write `verification.md`; push and open the PR after fix-agent-start-path, show-both-usage-windows and fix-usage-refresh. Check passed (380 tests) and verification written (2026-10-23); push waits for the owner.
