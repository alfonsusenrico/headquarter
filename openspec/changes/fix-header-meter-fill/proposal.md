## Why

The owner saw a Claude Code session at 96% context whose header meter looked about two-thirds
full. The meter's track is 60 px wide, but its fill is computed against 40 px, so every value
draws at two-thirds of its size. The two drifted apart when the header grew roomier (9b6eb3b).

Approved by the owner on 2026-10-27.

## Expected Outcome

The context meter fills the same share of its track as the context used: 96% used fills 96%
of the bar, and anything at or over 100% fills it completely.

## What Changes

- The meter's fill and track use one width constant in `crates/hq-app/src/workspace/header.rs`.
- The session details header requirement gains a scenario for the meter's fill.

## Non-goals

- Marking where Claude Code auto-compacts. Claude Code compacts before the window is full, at
  a threshold it does not report to Headquarter.
- Any change to the figures the header shows.

## Capabilities

- Modified: `terminal-pane` (session details header). No new project capability.

## Impact

The desktop app only. No protocol, host or phone change.
