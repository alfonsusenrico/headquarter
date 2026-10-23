# Verification

Revision: branch `feat/agy-usage` (main c858c9d with fix-agent-start-path,
show-both-usage-windows and fix-usage-refresh merged in), working tree with this change,
2026-10-23.

## Research evidence

`research/evidence/agy-usage/2026-10-23-headless-usage.md`: agy 1.4.4's
`agy -p "/usage" --output-format json` in an empty folder, 8.4 s, no conversation,
no tokens, nothing added to agy's input history.

## Checks

| Check | Result |
| --- | --- |
| `agy_usage_gives_each_pool_its_windows`, `a_failed_or_empty_agy_answer_has_no_usage`, `an_agent_without_a_usage_source_gets_one_plain_line` (`hq-signals`) | passed |
| `agy_rings_follow_the_pool_nearest_its_limit` (`hq-protocol`) | passed |
| `agy_usage_comes_from_its_own_usage_command` (host, stub agy: refresh, pools, runs in `quota/agy-workspace`) | passed |
| host tests (36) with `agy` stubbed wherever usage is read; no `/tmp/hqh-*` left | passed |
| `hq-probe quota` against the installed agy 1.4.4 | five windows read (Gemini 5h/week, Claude and GPT 5h/week, Argon week); agy's conversations 213 before and after |
| `./scripts/check` (fmt, clippy -D warnings, tests, cargo-deny, OpenSpec) | passed, 380 tests, 18 OpenSpec items |

`openspec validate --strict` notes that "Usage failures point at the provider" exists
only once fix-agent-start-path is archived; archive that change first.

## Not verified

- On screen (task 2.2): waits for the owner's test build.
- A hosted agy turn ending triggers a read 10 s later: covered by the same path as Codex
  (`codex_turn_ended`), not by its own test.

## On screen and final check (2026-10-23)

- The owner confirmed it on the test build (`test/combined-build`).
- `./scripts/check` on the branch as pushed: passed, 380 tests, 18 OpenSpec items (at a1cf286).
