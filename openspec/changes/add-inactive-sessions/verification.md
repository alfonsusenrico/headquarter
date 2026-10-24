# Verification

Date: 2026-10-24, branch `feat/inactive-sessions`. `./scripts/check` passed: 427 tests,
clippy `-D warnings`, cargo-deny, OpenSpec strict.

## Acceptance

| Criterion | Evidence | Result |
| --- | --- | --- |
| A quit or a crash keeps the session in the sidebar, inactive; its shell ends | `quitting_inside_or_a_crash_keeps_the_session_inactive`; owner on screen | passed |
| ✕ still closes and removes; on an inactive row it asks "Remove …?" | `closing_quits_with_the_agents_keys_and_leaves_the_sidebar`; owner on screen | passed |
| Inactive rows: moon, dimmed title and age; the pane offers Resume Session and starts nothing until pressed | sidebar model tests; owner on screen | passed |
| Sessions running at a crash, reboot or `hq stop` start with the next host; inactive ones stay | `a_host_crash_restarts_what_ran_with_the_agent_resume`, `stop_ends_running_sessions_and_the_next_host_starts_them`, `only_sessions_running_at_a_crash_start_again`; owner on screen (alpha started with the host) | passed |
| One at a time in sidebar order; a session asked for starts ahead; "starting" until ready | `sessions_start_with_the_host_in_sidebar_order_and_failures_stay_inactive`, `a_session_asked_for_starts_ahead_of_its_turn`, `sessions_starting_with_the_host_breathe_until_ready`, `sessions_start_in_the_order_the_sidebar_shows_them` | passed |
| A start failure leaves the session inactive with its reason; told once | the order test (deleted folder); `a_start_failure_is_told_once`, `the_last_start_failure_told_is_kept` | passed |
| Upgrade marks only the sessions the last stop ended | `upgrade_marks_only_what_the_last_stop_ended` | passed |
| A Claude session that never had a message starts afresh under its id | `a_host_crash_restarts_what_ran_with_the_agent_resume` (`--session-id`); owner on screen (gamma) | passed |
| A missing folder offers Remove from Sidebar | owner on screen (beta) | passed |
| `hq ls` says "inactive" and "starting"; `hq resume --interrupted` is gone | `inactive_and_starting_sessions_are_named` | passed |
| An "ext" session closed elsewhere becomes inactive | `closed_in_the_other_terminal_becomes_inactive` | passed |

## Not verified

- The start-failure notification on screen: macOS shows notifications only from the
  installed app, not a dev build.
- The pacing with several real agents at once (the host test uses stubs).
- Linux.
