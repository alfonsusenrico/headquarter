# Tasks

## 1. Index and host

- [x] 1.1 Migration `0008` (design D5); `shutdown` marks live sidebar sessions before ending them; an exit that is not a close keeps the session in the sidebar, inactive, and ends its shell. Verify with store tests (migration marks only the last stop's sessions; older marks cleared) and host tests (`/exit` stub keeps the row inactive; `hq stop` marks; ✕ still removes).
- [x] 1.2 Start-up queue (D3): marked sidebar sessions start one at a time, a client `Resume` jumps the queue, conversations open elsewhere are skipped, failures leave the session inactive with `start_error` (D6). Verify with host tests using stub agents: three marked, one inactive, one with a deleted folder; order and pacing; a `Resume` mid-queue.
- [x] 1.3 Protocol: `start_error`, `Dismiss` removed; CLI `hq ls` words "inactive" and "starting", `hq resume --interrupted` removed. Verify with protocol round-trip and render tests.

Done 2026-10-24: store tests (`what_was_running_is_marked_to_start_and_starting_or_skipping_clears_it`, `upgrade_marks_only_what_the_last_stop_ended`), host tests (`a_host_crash_restarts_what_ran_with_the_agent_resume`, `stop_ends_running_sessions_and_the_next_host_starts_them`, `sessions_start_with_the_host_in_sidebar_order_and_failures_stay_inactive`, `a_session_asked_for_starts_ahead_of_its_turn`, `only_sessions_running_at_a_crash_start_again`, `quitting_inside_or_a_crash_keeps_the_session_inactive`), `sessions_start_in_the_order_the_sidebar_shows_them`, protocol and render tests pass.

Expected Result: `cargo test -p hq-store -p hq-daemon -p hq-protocol -p hq-cli` passes.

## 2. App

- [x] 2.1 Moon glyph and dimmed inactive rows; "starting" breathing on every session the host will start until its agent is ready; Resume Session pane (Enter presses it); ✕ on inactive asks "Remove “title” from the sidebar?"; the shown session is resumed first; the interrupted note removed. Verify with sidebar model tests and on screen.
- [x] 2.3 Never-used Claude sessions start afresh under their id (D7); the inactive pane offers "Remove from Sidebar" when the folder is gone (D8). Verify with the host test `a_host_crash_restarts_what_ran_with_the_agent_resume` (unsaved → `--session-id`) and on screen.
- [x] 2.2 Start-failure notification, told once across app restarts. Verify with a notifications unit test and on screen (a session whose folder was renamed while the host was stopped).

Done 2026-10-24: `sessions_starting_with_the_host_breathe_until_ready`, `closed_in_the_other_terminal_becomes_inactive`, `a_start_failure_is_told_once`, `the_last_start_failure_told_is_kept` pass; on screen the owner confirmed quit to inactive, Resume Session, the remove question, starting with the host, a never-used session resuming, and Remove from Sidebar for a missing folder ("ok working"). The notification itself was not seen: macOS shows them only from the installed app.

Expected Result: the owner checks it on screen in a dev window: quit an agent, see the moon, resume it; stop the dev host and reopen, sessions start with the shown one first.

## 3. Delivery

- [x] 3.1 `./scripts/check` passes; `verification.md`; archive; push and open the PR for the owner to merge. No release (owner, 2026-10-24). Done: `./scripts/check` passed, 427 tests (2026-10-24).

Expected Result: the PR is open.
