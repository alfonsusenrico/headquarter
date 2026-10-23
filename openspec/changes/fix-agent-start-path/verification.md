# Verification

Revision: branch `fix/agent-start-path` as pushed, 2026-10-23.

## Checks

| Check | Result |
| --- | --- |
| `an_agent_runs_with_its_own_folder_first_then_the_login_path` (`hq-discovery`) | passed |
| host tests, incl. `a_folder_trust_question_stops_the_usage_read_unanswered`; usage tests stub `codex` and leave no `/tmp/hqh-*` | passed |
| `an_agent_without_a_usage_source_gets_one_plain_line` (`hq-signals`), `hq usage` render test | passed |
| `./scripts/check` | passed, 373 tests, 15 OpenSpec items (at a31c38d) |

## On screen

The installed test build read Codex's usage at start (host log, 30 successful reads,
2026-10-22 and 2026-10-23); the owner confirmed setup and the usage details.

## Not verified

- A Codex session started from the installed app (part of task 1.3): none was started
  in Headquarter during the test.
