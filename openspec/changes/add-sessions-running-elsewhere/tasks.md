# Tasks

## 1. Add sessions

- [x] 1.1 A linked outside session is pickable, tagged, not greyed; an unlinked one stays blocked. Verify with `rows_show_title_folder_and_whether_they_can_be_added` (the app has no window-level tests; picking goes through the same `addable` flag and the host's add request, which already accepts linked outside sessions). Done: `cargo test -p hq-app` 132 passed (2026-10-23).

Expected Result: Add sessions adds a session running in another terminal; `cargo test -p hq-app` passes.

## 2. Pane

- [x] 2.1 "Closed in the other terminal." with "Open here" when a shown session stops running elsewhere; forgotten when another session is chosen or it is opened. Verify with `closing_the_other_terminal_is_told_apart` (the transition, including "nothing started": a live update is not taken for it). Done: `cargo test -p hq-app` 132 passed (2026-10-23).

Expected Result: closing the outside agent leaves the pane waiting with "Open here"; `cargo test -p hq-app` passes.

## 3. Found on screen

- [ ] 3.1 Rows that cannot be picked keep the tick's space. Verify on screen (window capture).
- [x] 3.2 The host reports unlinked outside agents as they appear and as no longer offered when they link or exit. Verify with a host unit test of the scan comparison and an integration test with a stub agent that starts unlinked and exits. Done: `clients_hear_unlinked_agents_appear_change_and_go` and `clients_hear_outside_agents_come_and_go` pass; the integration test fails without the host change (timed out waiting for the change), 2026-10-23.

Expected Result: no stale "pid N" rows; `cargo test` passes.

## 4. On screen and delivery

- [ ] 4.1 On screen with a real Claude Code session in another terminal: add it, see "ext", close it there, see "Closed in the other terminal.", press "Open here". Evidence: window captures in `research/evidence/add-sessions-running-elsewhere/`.
- [ ] 4.2 `./scripts/check` passes; `verification.md` written.
- [ ] 4.3 After the owner accepts: sync and archive the change, push the branch, open the one PR, and report its URL and the verification record.

Expected Result: the owner has accepted the behaviour on screen and the PR is open for the owner to merge.
