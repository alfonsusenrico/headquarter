# Tasks

## 1. Status word

- [x] 1.1 `shell` reads as working. Verify with `claude_agents_statuses` and the host test `claude_status_comes_from_its_records_without_starting_claude_each_scan` (record says "shell"), which fails without the change. Done: both pass with it; the host test timed out without it (2026-10-24).

Expected Result: background commands keep a hosted Claude session working.

## 2. On screen and delivery

- [ ] 2.1 Real check: a hosted Claude Code session in a dev host runs `sleep 40` in the background; its status stays working until it ends. Evidence: the status timeline.
- [ ] 2.2 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge.

Expected Result: the PR is open.
