# Claude usage without a five-hour window

Status: plan, waiting for the owner's approval.

## Why

The owner (2026-11-02): Claude usage stays stuck ("Couldn't update since…", "Claude Code didn't
report its usage"); Refresh does not help. Since Claude Code 2.1.316 (2026-10-31) neither its
`/usage` screen nor its status line reports a five-hour window, only the week (all models and per
model). Headquarter rejects a screen without a five-hour block, and shows a stale five-hour row as
"reset before this reading" (`research/evidence/claude-usage-week-only/findings.md`).

## Expected Outcome

- Claude usage updates again on 2.1.316: the week, the week per model, and their resets.
- A five-hour window shows when Claude Code reports one, and no row otherwise.

## What Changes

- The `/usage` screen parser needs one all-models window (D1).
- Claude's five-hour window is no longer a standing window (D2).
- `provider-quota` spec.

## Non-goals

- Other agents' windows (unchanged).

## Capabilities

No new project capability. Modified: `provider-quota`.

## Impact

`hq-signals` (`claude_usage.rs`, `quota.rs`), `hq-protocol` usage rows follow. No protocol change.
Checked against Claude Code 2.1.316.
