# Tasks

## 1. Host

- [x] 1.1 `RemoveConversation` and `removal_ways`; the host trashes Claude transcripts, runs Codex archive and delete, refuses running sessions and other ways, and stops offering the row. Verify with `past_conversations_are_removed_the_way_their_agent_allows` (host, `HQ_TRASH_DIR`). Done (2026-10-23).

## 2. Add sessions

- [x] 2.1 The ⋯ menu (slot kept), Undo toast with delayed send, sending on close, Delete confirmation. Verify with `rows_offer_the_removals_their_agent_allows_unless_running` (`hq-app`) and clippy. Done (2026-10-23).
- [ ] 2.2 On screen: Move to Trash with Undo and without; Codex Archive; Delete permanently asks; agy rows aligned with no menu. Evidence: the owner's check.

Expected Result: the owner can clean out past conversations from Add sessions.

## 3. Delivery

- [ ] 3.1 `./scripts/check` passes; write `verification.md`; push and open the PR after name-untitled-conversations and fix-agent-start-path. Check passed (386 tests) and verification written (2026-10-23).
