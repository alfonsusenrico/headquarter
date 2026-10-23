# Verification

Revision: branch `feat/offer-continuable-sessions` (fix-agy-link, hide-gone-folders and
fix-feed-missed-changes merged in), working tree with this change, 2026-10-23.

## Diagnosis evidence (owner's machine, metadata only)

- Claude Code: 41 top-level transcripts with entrypoint `cli`, 2 with `sdk-cli`; 12
  without a prompt or reply, 5 of them `continued-in` stubs; 152
  sub-agent transcripts stored under `<session>/subagents/` (never imported); 5 index rows
  whose transcript Claude Code had deleted.
- Codex: 82 rollouts: 55 `{"subagent": {"other": "guardian"}}`, 6 `thread_spawn`
  sub-agents, 2 `exec`, 12 `vscode`, 7 `cli`; 6 index rows whose rollout is gone.
- agy: 63 sub-agent conversations (nesting 1 or more, never imported); 16 top-level
  conversations with `step_count` 0.

## Checks

| Check | Result |
| --- | --- |
| `program_runs_and_sub_agents_are_told_apart`, `agy_reads_top_level_rows_without_previews` (`hq-signals`) | passed |
| `past_conversations_are_offered_only_while_kept_and_owner_started` (host) | passed |
| `a_host_that_predates_offering_counts_as_offering_everything` (`hq-protocol`) | passed |
| `conversations_whose_folder_is_gone_are_not_offered` (`hq-app`) | passed |
| `./scripts/check` (fmt, clippy -D warnings, tests, cargo-deny, OpenSpec) | passed, 382 tests, 21 OpenSpec items |

`openspec validate --strict` notes that the modified requirement exists once
hide-gone-folders is archived; archive that change first.

## Not verified

- On screen (task 1.4): waits for the owner.

## On screen and final check (2026-10-23)

- The owner confirmed it on the test build (`test/combined-build`).
- `./scripts/check` on the branch as pushed: passed, 382 tests, 21 OpenSpec items (at 66e6ccc).
