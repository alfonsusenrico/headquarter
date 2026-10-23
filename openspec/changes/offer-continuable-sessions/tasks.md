# Tasks

## 1. Which conversations are offered

- [x] 1.1 Readers say whether the owner started each conversation; Codex sub-agent rollouts are skipped. Claude transcripts without a prompt or reply (`continued-in` stubs) are not offered. Verify with `program_runs_and_sub_agents_are_told_apart` and `agy_reads_top_level_rows_without_previews` (`hq-signals`). Done (2026-10-23).
- [x] 1.2 The host keeps the offered set per pass, reports `offered` per row and broadcasts changes. Verify with `past_conversations_are_offered_only_while_kept_and_owner_started` (host). Done (2026-10-23).
- [x] 1.3 `SessionInfo.offered` defaults to true; Add sessions keeps only offered sessions. Verify with `a_host_that_predates_offering_counts_as_offering_everything` (`hq-protocol`) and `conversations_whose_folder_is_gone_are_not_offered` (`hq-app`). Done (2026-10-23).
- [ ] 1.4 On screen: the untitled deleted Claude rows, `codex exec` runs and empty agy conversations are gone from Add sessions; the counts drop. Evidence: the owner's check.

Expected Result: Add sessions offers only the owner's own continuable conversations, for every agent.

## 2. Delivery

- [ ] 2.1 `./scripts/check` passes; write `verification.md`; push and open the PR after fix-agy-link, hide-gone-folders and fix-feed-missed-changes. Check passed (382 tests) and verification written (2026-10-23).
