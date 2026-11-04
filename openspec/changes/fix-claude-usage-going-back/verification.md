# Verification: fix-claude-usage-going-back

## Revision tested

Branch `fix/claude-usage-going-back` at 6956679 (on `main` 4deda3d), the owner's MacBook Air.

## Checks

| Check | Result |
| --- | --- |
| `./scripts/check` (fmt, clippy `-D warnings`, tests, cargo-deny, Android, OpenSpec) | passed: 617 tests, 0 failed; OpenSpec 19/19 |
| `hq-signals` `windows_of_one_period_reset_within_the_hour` | the `/usage` screen's minute reset matches the status line's second reset; another week, length, model or no reset does not |
| `hq-signals` `an_older_status_line_report_is_not_saved` | 59% saved; an older 58% not saved (`read_at` kept); 59% again saved with a new `read_at`; 60% saved; a new week at 2% saved |
| `hq-signals` `the_higher_share_of_one_period_wins` | 58% against `/usage` 59% gives 59%; 60% stays 60%; a new week stays its own |
| `hq-daemon` `a_newer_status_line_never_lowers_the_week` | spec scenarios "idle session redraws" (59%), "reports more" (60%), "a new week" (2%) |
| `hq-daemon` `a_newer_usage_read_is_shown_as_read` | spec scenario "Refresh reads a lower share" (57%) |
| `hq-cli` host test `an_idle_sessions_older_limits_do_not_replace_newer_ones` | the real `hq hook-statusline` helper with status-line JSON: 59, then 58 keeps 59, 60, a new week 2 |
| Regression: the fix turned off (`is_behind` ignored, `higher_of_same_period` returning its window) | 3 of the new tests fail, as expected |

All data synthetic: stand-in status-line JSON, no real Claude Code session.

## Not verified

- A hosted host with a stub Claude whose status line and `/usage` read interleave: the merge (D2)
  is covered by `claude_usage()` tests, and the helper (D1) through the real binary.
- The owner's daily use with several idle Claude sessions on the installed app.
