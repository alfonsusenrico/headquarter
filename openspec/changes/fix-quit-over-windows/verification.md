# Verification: Quit's question over every window

Tested: branch `fix/quit-over-remote` @ 8c37b00 from `main` @ 26bf33d. 2026-10-28.

## Commands

- `./scripts/check`: passed. Rust 541 tests in 27 suites, 0 failed. fmt, clippy `-D warnings`,
  cargo-deny, Android checks, OpenSpec strict 19/19.
- No automated test reaches the workspace's drawing order (the workspace needs a host). The fix
  is checked on screen.

## On screen

macOS dev build, empty test data folder, a stand-in agent configured. Captures:
`research/evidence/fix-quit-over-windows/`.

- Remote Control open, ⌘Q: the question shows above the window. Esc cancels it, and a second Esc
  closes Remote Control (before the fix the question was hidden and Esc only reached it).
- Window ▸ Usage open, ⌘Q: the question shows above it. Esc cancels, Esc closes the Usage
  window.
- Check for Updates' notice ("Headquarter 0.1.16 is the latest version."), ⌘Q: the question
  shows above it, and Esc cancels it.

Found, not part of this change: the update notice has no Esc handling, so Esc never closes it,
with or without Quit. OK or Enter closes it.

## Not verified

- The update window while an update downloads or installs (the question was checked over the
  notice only).
- Linux.

## Owner check

Pending.
