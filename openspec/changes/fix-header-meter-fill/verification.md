# Verification: the context meter fills to its full width

Tested: branch `fix/header-meter-fill` @ 9c002ed, from `main` @ 33f0a08. 2026-10-27.

## Commands

- `./scripts/check`: passed. Rust 510 tests, 0 failed, app and Android checks, OpenSpec 19 items,
  0 failed. A stale `hq-config` test binary built under the repository's old folder failed three
  fixture tests on the first run; after `cargo clean -p hq-config` every test passed.
- Unit test `the_fill_matches_the_percentage_of_the_track`: 96% fills 57.6 of 60 px, 0% fills
  nothing, and 140% fills the whole track.

## Real app (owner, 2026-10-27)

A local release build of `main` with this change, `fix-claude-conversations` and
`add-quit-stops-host` (`test/real-case` @ edc9f3f). The owner checked a Claude Code session's
meter and accepted it ("works").

## Not verified

- Codex's meter on screen: it uses the same `render_meter`.
