# Verification

Date: 2026-10-24, branch `feat/session-shell-panel`.

## Acceptance

| Criterion | Evidence | Result |
| --- | --- | --- |
| Cmd-T opens a shell in the session's folder under the agent; the sidebar is unchanged | Owner, dev window | passed |
| The shell keeps running while another session is shown, and reattaches with its output | `a_sessions_shell_runs_beside_it_and_ends_with_it`; owner on screen | passed |
| Cmd-T again, or `exit`, closes the panel and ends the shell | Host test (`CloseShell` gives `Ended`; a fresh shell starts empty); owner on screen | passed |
| Closing or removing the session ends its shell at once | Host test (`Remove` gives `Ended`); owner on screen | passed |
| The shell has no status, index row or scans; the session's status is unaffected | Host test (status stays Done); owner on screen with `claude` run inside the shell | passed |
| Divider keeps both parts usable; open panels and the split are remembered | `the_divider_keeps_both_parts_usable`, `shell_panels_and_their_share_are_kept_with_the_rest` | passed |
| Plain Ctrl-T still reaches the agent | `the_shell_panel_takes_cmd_t_or_ctrl_shift_t_only` | passed |
| Resource cost with many shells | `research/evidence/session-shell-panel/resources.md`: 7.5-8.1 MB per shell, no idle CPU, host memory unchanged | passed |

## Not verified

- Ctrl-Shift-T on a real Linux desktop (unit-tested mapping only).
