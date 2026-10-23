# Proposal

## Why

Each agent's footer ring showed only its tightest window, so a nearly spent week hid
behind a quiet five hours (and the other way round). On 2026-10-22 the owner asked to
see both the daily and the weekly window.

## Expected Outcome

- Each agent's footer mark has two rings: inside, its shortest window (Claude Code's and
  Codex's 5 hours); outside, its longest (the week).
- Each ring, and each bar in the details, turns red on its own at 80% used: the
  theme's terminal red, apart from the amber of "needs you".
- The label under the rings appears from 80%: one agent near its limit is named with
  its nearest window ("claude 92% · week"; on equal whole percentages the 5-hour window,
  which resets sooner); two or more read "2 agents near their limit".
- The tooltip names every window: "Claude: 5h 42%, week 67% used".

## What Changes

- `hq-protocol` (`usage.rs`): `rings()` picks the shortest and longest all-models
  windows that still hold. Per-model windows stay in the details.
- `hq-app`:
  - `glyph::usage_ring` draws an inner ring when there are two windows, with a smaller
    mark inside;
  - the footer gives each ring its own colour and lists every window in the tooltip.

Decisions (owner, 2026-10-22): **two rings, one inside the other** (A of five options in
the local preview). Rejected: two rings side by side, a ring plus a week bar, a split
ring, labelled bars.

Decisions (owner, 2026-10-23): **one threshold, 80%, for colour and label**; a
**hybrid label**, naming a single agent and counting several. Rejected: naming every
agent near its limit (long line), naming both windows of one agent (the nearest is
enough), the 50% label and 90% amber of v0.1.0. **The colour is the theme's terminal
red** rather than a new editable colour, so theme files and the theme editor are
unchanged.

## Non-goals

- The usage details panel's layout, which already lists every window.

## Capabilities

### Modified Capabilities
- `provider-quota`: the usage strip shows both windows.

## Impact

`crates/hq-protocol/src/usage.rs`, `crates/hq-app/src/sidebar/glyph.rs`,
`workspace.rs`.

## Approval

Approved by the owner on 2026-10-22 ("5. A").
