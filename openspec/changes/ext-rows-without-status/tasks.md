# Tasks

## 1. Rows

- [x] 1.1 `needs_you` ignores sessions running elsewhere; their rows draw no glyph or status word; no notification. Verify with updated `needs_you_then_groups_in_order_then_ungrouped`, a sidebar model test, and a notifications test. Done: `needs_you_then_groups_in_order_then_ungrouped`, the sidebar model test (working elsewhere shows no status word), `a_session_running_elsewhere_never_notifies` (replaces `outside_sessions_notify_like_hosted_ones`), and the host test `outside_sessions_are_linked_and_block_resume` (no folder needs-you count) pass, 2026-10-23.

Expected Result: `cargo test` passes.

## 2. On screen and delivery

- [ ] 2.1 On screen: an "ext" row working in another terminal shows only "ext" and its age; after closing it there and opening it here, its status shows. Evidence: the owner's confirmation.
- [ ] 2.2 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge.

Expected Result: the owner accepts the rows and the PR is open.
