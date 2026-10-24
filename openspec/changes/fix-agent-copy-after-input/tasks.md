# Tasks

## 1. Copy

- [x] 1.1 Agent copies only within 2 seconds of the owner's input. Verify with `agent_copies_count_only_right_after_the_owners_input`. Done: passes; hq-app 150 tests, clippy clean (2026-10-24).

Expected Result: copies re-sent on a redraw are ignored.

## 2. On screen and delivery

- [ ] 2.1 On screen: copy in session A, copy in session B, open A again, paste in Notes: B's text, no note on opening. Evidence: the owner's confirmation.
- [ ] 2.2 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge.

Expected Result: switching sessions no longer changes the clipboard, and the PR is open.
