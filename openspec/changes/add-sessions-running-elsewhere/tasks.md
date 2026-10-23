# Tasks

## 1. Add sessions

- [x] 1.1 A linked outside session is pickable, tagged, not greyed; an unlinked one stays blocked. Verify with `rows_show_title_folder_and_whether_they_can_be_added` (the app has no window-level tests; picking goes through the same `addable` flag and the host's add request, which already accepts linked outside sessions). Done: `cargo test -p hq-app` 132 passed (2026-10-23).

Expected Result: Add sessions adds a session running in another terminal; `cargo test -p hq-app` passes.

## 2. Pane

- [x] 2.1 "Closed in the other terminal." with "Open here" when a shown session stops running elsewhere; forgotten when another session is chosen or it is opened. Verify with `closing_the_other_terminal_is_told_apart` (the transition, including "nothing started": a live update is not taken for it). Done: `cargo test -p hq-app` 132 passed (2026-10-23).

Expected Result: closing the outside agent leaves the pane waiting with "Open here"; `cargo test -p hq-app` passes.

## 3. On screen and delivery

- [ ] 3.1 On screen with a real Claude Code session in another terminal: add it, see "ext", close it there, see "Closed in the other terminal.", press "Open here". Evidence: window captures in `research/evidence/add-sessions-running-elsewhere/`.
- [ ] 3.2 `./scripts/check` passes; `verification.md` written.
- [ ] 3.3 After the owner accepts: sync and archive the change, push the branch, open the one PR, and report its URL and the verification record.

Expected Result: the owner has accepted the behaviour on screen and the PR is open for the owner to merge.
