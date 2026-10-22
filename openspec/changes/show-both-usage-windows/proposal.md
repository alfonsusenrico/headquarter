# Proposal

## Why

Each agent's footer ring showed only its tightest window, so a nearly spent week hid
behind a quiet five hours (and the other way round). On 2026-10-22 the owner asked to
see both the daily and the weekly window.

## Expected Outcome

- Each agent's footer mark has two rings: inside, its shortest window (Claude Code's and
  Codex's 5 hours); outside, its longest (the week).
- Each ring turns amber on its own when it passes the high mark.
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

## Non-goals

- The usage details panel, which already lists every window.
- Which agent counts as nearest its limit, and the label: unchanged.

## Capabilities

### Modified Capabilities
- `provider-quota`: the usage strip shows both windows.

## Impact

`crates/hq-protocol/src/usage.rs`, `crates/hq-app/src/sidebar/glyph.rs`,
`workspace.rs`.

## Approval

Approved by the owner on 2026-10-22 ("5. A").
