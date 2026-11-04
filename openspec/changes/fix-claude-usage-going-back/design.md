# Design: Claude usage never goes back within a window

## Context

Two sources feed Claude's usage. The status-line helper writes `quota/claude.json` on every
status-line run of a hosted session, with `read_at` set to the time it ran. The hidden `/usage`
read keeps its reading in the host. `claude_usage()` takes the five-hour and weekly windows from
the newer of the two. Claude Code runs the status line on events that bring no new numbers (resume,
permission mode, `/compact`, prompt-cache expiry), so `read_at` can be newer than the numbers.

Inside one window (one `resets_at`), the share used only grows: Anthropic counts usage until the
window resets. A lower share for the same window is an older figure.

## D1. The helper does not save a stale report

Before saving, the helper reads the saved snapshot. A report whose share is lower than the saved
one for the same window (five hours or week, resets within an hour of each other) is stale: all of
a report's windows come from one API response, so the whole report is not saved. Any other report
is saved as today, with `read_at` = now, also when its numbers equal the saved ones (a busy
session with unchanged numbers keeps the reading fresh). A report for a new period is saved.

Same window means resets within an hour of each other: Claude Code's `resets_at` is an epoch
second and the `/usage` screen gives minutes; the next five-hour window resets at least five hours
after the last one, so an hour tells periods apart.

Rejected:
- Keeping each session's last report and ignoring unchanged ones (a file per session, and stale
  when two sessions alternate).
- Taking the highest share ever (stuck until the reset if Claude Code ever lowers it).
- Saving only reports whose numbers changed (a busy session with unchanged numbers would look
  overdue after 15 minutes).

## D2. The host prefers the higher share of the same window

In `claude_usage()`, when the status-line reading is newer than the `/usage` reading, each of its
windows is compared with the `/usage` window of the same kind: for the same window, the higher
share is shown; for a new window, the status line's. When the `/usage` reading is newer, it is
shown as read, also when lower: Refresh stays the owner's way to see exactly what Claude Code
reports.

## Failure behaviour

- An unreadable or missing snapshot: the helper writes the report as today.
- A window without `resets_at`: compared as not the same window (taken as reported).

## Known limit

If Anthropic lowers a share inside a window (for example a raised limit), status-line reports keep
the higher number until a Refresh or the reset.

## Risks

Two helpers writing at once: the snapshot is replaced atomically, and the comparison runs again on
the next report, so the worst case is one report lost, never a lower figure kept.
