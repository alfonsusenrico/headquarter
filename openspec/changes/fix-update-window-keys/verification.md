# Verification: the update window takes the keyboard

Tested: branch `fix/update-notice-esc` @ 120cedf from `main` @ 3dce1f3. 2026-10-28.

## Commands

- `./scripts/check`: passed. Rust 542 tests in 27 suites, 0 failed. fmt, clippy `-D warnings`,
  cargo-deny, Android checks, OpenSpec strict 19/19.
- New test: `escape_closes_only_what_offers_a_way_out` (each phase).

## On screen

macOS dev build on a test host with one session whose stand-in agent prints every key it
receives. Captures: `research/evidence/fix-update-window-keys/`.

- Typed "x" in the session: the agent received it.
- Check for Updates…: "Headquarter 0.1.16 is the latest version." Typed "abc", then Escape: the
  notice closed and the agent received neither "abc" nor Escape.
- Typed "y": the agent received it, so the terminal had the keyboard back.
- With the notice open, ⌘Q: the quit question showed above it. Escape cancelled the question,
  Escape closed the notice, and "z" reached the agent.

## Not verified

- Escape in the offer, "Restart to finish updating", a failed update and the wait after
  installing (covered by the unit test, not reached on screen without a newer release).
- Escape doing nothing while an update installs (unit test only).
- Linux.

## Owner check

Pending.
