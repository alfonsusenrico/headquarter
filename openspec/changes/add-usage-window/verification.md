# Verification: the Usage popover and window

Tested: branch `feat/usage-window` @ bdb7873 from `main` @ d8a63b9, working tree with the design
and tasks updates. 2026-10-28.

## Commands

- `./scripts/check`: passed. Rust 533 tests in 27 suites, 0 failed. fmt, clippy `-D warnings`,
  cargo-deny (advisories, bans, licenses, sources ok). `openspec validate --all --strict`:
  19 items, 0 failed.
- The change's own tests, all passing in that run:
  - `hq-protocol`: `resources_round_trip`.
  - `hq-signals`: `a_tree_counts_what_its_root_started`,
    `the_first_sample_has_no_cpu_and_gone_processes_have_nothing`, `figures_add_up`.
  - `hq-cli` host test: `resources_cover_what_a_session_started_and_only_running_sessions` (a
    stub agent starts a busy child: at least 2 processes, CPU above 50% on the second sample,
    a killed session has no entry). The refused-for-phones list in `remote.rs`'s test.
  - `hq-app`: `usage_is_cmd_u_and_ctrl_u_stays_with_the_agent`,
    `the_table_lists_sidebar_sessions_and_sorts_by_figures`,
    `the_popover_shows_the_busiest_running_sessions`, `totals_and_headquarter_add_up`,
    `figures_read_like_activity_monitor`.

## On screen

macOS 27.0.1, dev build of bdb7873, isolated data and config folders (`HQ_DATA_DIR`,
`HQ_CONFIG_DIR`). Stand-in agents only: a claude stand-in that starts three child loops, one of
them busy, two idle codex stand-ins, and a stopped agy session. Window captures:
`research/evidence/usage-window/`.

- ⌘U opens the popover. It shows "…" for CPU at first, figures after the next refresh: the
  busy stand-in at 101–102% with "+5 child processes", the idle ones at 0.0%, Headquarter at
  1–3%. The stopped session is not in the top five. ⌘U again and Esc close it.
- The popover stays open after a click on a sidebar session, over the running terminal.
- Show all and Window ▸ Usage open the window: the Headquarter card ("app x · y, host x · y"),
  All agents ("3 running, 1 not running"), and the table in sidebar order. The agy row is
  dimmed, reads "not running" and has no figures.
- Clicking CPU and Memory sorts by them with ↓, the stopped session last. Clicking Session
  returns to sidebar order.
- Esc, the ✕ and a click outside close the window. Typing afterwards reaches the terminal.
- Dark and light themes.
- Measuring only while shown: the host's CPU time (`ps -o time`) over 20 s grew 0.04 s with
  both closed, 0.29 s with the popover open, 0.03 s closed again.

## Not verified

- Linux: Ctrl+Shift+U and the figures there (no Linux build locally). The sampler skips
  Linux threads, covered by code review only.
- Real agents. The stand-ins exercise the same process tree a real agent has.
- A session with its shell panel open (its shell counts in the session's figures, covered by
  the host code, not on screen).
- More than five running sessions in the popover (covered by the unit test).

## Owner check

Pending.
