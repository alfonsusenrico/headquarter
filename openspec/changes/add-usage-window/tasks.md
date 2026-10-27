# Tasks

## 1. Host

- [x] 1.1 `Resources` request and response, protocol 13, refused for phones (D1, D5). Evidence: protocol round-trip test, `allowed()` test. Done 2026-10-27: `resources_round_trip`, `Resources` in `allowed()`'s refused list and its test.
- [x] 1.2 Sampling a session's processes and descendants, CPU per core on the second sample (D1). Evidence: host test with a stub agent that starts a child process and burns CPU: the child is counted, CPU rises above 50% on the second sample, a stopped session has no entry. Done 2026-10-27: `hq_signals::resources::Sampler` (tests: a tree counts what its root started, no CPU on the first sample), host test `resources_cover_what_a_session_started_and_only_running_sessions`.

## 2. App

- [x] 2.1 The popover on ⌘U / Ctrl+Shift+U with the top five and Show all (D4). Evidence: unit test of the top-five order, the dev app on screen. Done 2026-10-28: `the_popover_shows_the_busiest_running_sessions`, `usage_is_cmd_u_and_ctrl_u_stays_with_the_agent`; on screen in both themes (`research/evidence/usage-window/`): "…" on opening, CPU after 2 s, ⌘U and Esc close it, it stays open over a running terminal.
- [x] 2.2 The Usage window: the Headquarter card, All agents, the sortable table, not running rows, child-process notes, and Window ▸ Usage on macOS (D3, D4). Evidence: unit tests of sorting and totals, the dev app on screen in both themes. Done 2026-10-28: `the_table_lists_sidebar_sessions_and_sorts_by_figures`, `totals_and_headquarter_add_up`, `figures_read_like_activity_monitor`; on screen in both themes: Show all and Window ▸ Usage open it, CPU, Memory and Session headings sort, the stopped agy row reads "not running", the busy stand-in reads "+5 child processes", Esc, ✕ and a click outside close it, focus returns to the terminal.
- [x] 2.3 Measuring only while shown (D2). Evidence: the host log or a counter shows no requests with both closed. Done 2026-10-28: the host's CPU time over 20 s grew 0.04 s closed, 0.29 s with the popover open, 0.03 s closed again (`ps -o time`); the refresh task belongs to the view, which closing drops.

Expected Result: the owner sees what each session and Headquarter itself cost.

## 3. Verification and delivery

- [x] 3.1 `./scripts/check` passes. `verification.md` written. Done 2026-10-28: 533 tests, 0 failed.
- [ ] 3.2 The owner checks it.
- [ ] 3.3 After the owner accepts: archive, push `feat/usage-window`, open the PR, report its URL.

Expected Result: one PR with the Usage window.
