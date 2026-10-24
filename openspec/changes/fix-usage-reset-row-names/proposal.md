# Proposal

## Why

The owner saw agy's usage details break "week · Gemini" into "· / Gemi / ni"
(2026-10-24). Rows with a bar kept a window's name on one line, as decided on
2026-10-21 (refresh-usage-on-activity), but rows without one ("reset since this
reading", "reset before…") gave the name a fixed 36 px that wraps. agy's per-model
window names made it visible.

## Expected Outcome

- Every row in usage details keeps the window name on one line; the bar, or the grey
  words of a reset row, take the remaining width.

## What Changes

- `hq-app` (`workspace.rs`): one `window_name_cell` draws the name for both row kinds;
  the reset words take the rest of the row.

## Non-goals

- The panel's width (still at least 300 px, the sidebar's width when wider, floating
  over the pane).

## Capabilities

### Modified Capabilities
- `provider-quota`: reset rows keep the window name on one line.

No project capability is added.

## Impact

`crates/hq-app/src/workspace.rs`. Seen with agy's `/usage` per-model windows.

## Approval

Approved by the owner on 2026-10-24 ("Approve").
