# Claude usage never goes back within a window

Status: approved by the owner 2026-11-04 ("approve, build it").

## Why

The owner (2026-11-04, Claude Code 2.1.318): the Claude week showed 58% while `/usage` in a
session showed 59%. Refresh brought 59%, and after a while of work the panel fell back to 58%,
while `/usage` had gone up to 60%.

A hosted session's status line carries that session's `rate_limits` from its own last API
response, and the helper saves them stamped with the time it ran
(`crates/hq-signals/src/helpers.rs`, `record_claude_limits`). Claude Code also runs the status
line without a new response: on resume, a permission-mode change, `/compact`, and when the
session's prompt cache expires ("When the status line updates",
https://code.claude.com/docs/en/statusline). An idle session therefore re-sends its older
numbers as the newest reading, and the host prefers the newest one over a fresher `/usage` read
(`crates/hq-daemon/src/host/usage.rs`, `claude_usage`). Evidence:
`research/evidence/claude-usage-going-back/findings.md`.

## Expected Outcome

- The Claude percentages never fall back while the same window lasts, whichever session redraws
  its status line.
- A new window (after its reset) shows its own, lower numbers at once.
- Refresh (`/usage`) still shows what Claude Code reports, also when that is lower.

## What Changes

- The helper does not save a status-line report that is lower than the saved one for the same
  window (D1).
- The host takes the higher share of the same window when the status line is newer than the
  `/usage` read (D2).
- `provider-quota` spec: a new requirement.

## Non-goals

- Codex and agy usage (each read by Headquarter itself, one source).
- Reading which session sent a report, or keeping per-session readings.

## Capabilities

No new project capability. Modified: `provider-quota`.

## Impact

`hq-signals` (`helpers.rs`, `quota.rs`), `hq-daemon` (`host/usage.rs`). No protocol or stored
format change: `claude.json` keeps its shape.
