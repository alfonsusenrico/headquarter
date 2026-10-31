# Verification: fix-phone-scroll

Working tree on `fix/phone-scroll` (committed with this record), 2026-10-31.

## Checks

- `./scripts/check`: passed. 569 tests, clippy `-D warnings`, rustfmt, cargo-deny, Android build
  (debug and unsigned release), ktlint, Android Lint, OpenSpec (19 items).
- New `hq-mobile` tests: `a_full_screen_agent_with_the_mouse_gets_wheel_reports_at_the_cell`
  (Claude Code 2.1.312's modes: SGR reports at the cell, both directions, clamped to the grid, ten
  at most, no local scroll); `a_full_screen_agent_without_the_mouse_gets_arrows` (normal and
  application cursor; old-encoding mouse mode gets arrows, as on the desktop). The history test
  still scrolls locally and sends nothing.

## End to end without a phone

Isolated test host (`HQ_DATA_DIR`, `HQ_CONFIG_DIR` under `/private/tmp/hqs`, relays off), a real
Claude Code 2.1.312 session in the trusted scratch folder `/private/tmp/hq-status-research/claude-shell`
with a one-line prompt printing numbers, opened by the stand-in phone on this branch's core
(`research/prototypes/phone-standin`, new `scroll` and `digest` commands; only a hash of the screen
was printed):

- `history=0` throughout: Claude draws full-screen, so the old path had nothing to scroll.
- Two hashes 3 s apart without a drag: equal. After `scroll 8 10 20`: changed, then stable. After
  `scroll -8 10 20`: changed again. The drag reaches Claude Code and moves its view.
- The screen after scrolling back down was not byte-identical to the first one; how Claude Code
  steps per wheel report is for the device check to judge.

## Not verified

- On a real phone (the owner's TECNO or the realme), with the owner's go: how the drag feels.
- Codex and agy on the phone (same path; Codex uses the alternate screen).

## Left behind

- The scratch conversation `c23c4069…` in Claude Code's history for that scratch folder (created by
  this check).
