# A total in the Usage popover and window

Status: plan, waiting for the owner's approval.

## Why

The owner (2026-10-31): the Usage menu should end with the total, so it is clear what Headquarter
and the dev sessions use together.

## Expected Outcome

- The Usage popover (⌘U) and the Usage window end with a total: Headquarter itself (app and host),
  all running sessions together, and their sum, each with CPU and memory.

## What Changes

- Popover and window: a total block at the bottom (D1).
- `desktop-app` spec.

## Non-goals

- Other processes on the computer, or a history of the figures.

## Capabilities

No new project capability. Modified: `desktop-app`.

## Impact

`hq-app` (`workspace/resources.rs`). The figures are the ones already measured; no host change.
