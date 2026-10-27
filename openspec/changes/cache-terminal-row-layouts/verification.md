# Verification: faster terminal drawing on the phone

Tested: branch `perf/terminal-row-cache` from `main` @ aec19d3, working tree with the change
(the temporary timing removed before the check). 2026-10-27.

## Commands

- `./scripts/check`: passed. Rust 504 tests, 0 failed. ktlint, Android Lint (warnings as
  errors), app unit tests `UpdateCheckTest` 3 and `TerminalRowTest` 3, debug and release APKs.
  OpenSpec 19 items, 0 failed.
- `openspec validate cache-terminal-row-layouts --strict`: valid.

## On the dev phone

Phone: realme RMX1921, debug build. Host: isolated test host (`/tmp/hqrc`), stand-in agent
`flood-agent` (synthetic output only), paired over n0's relay. Details, scripts and screenshots:
`research/evidence/cache-terminal-row-layouts/`.

| Load | main: draw / median frame | this change: draw / median frame |
|---|---|---|
| scrolling output (every row moves) | 108-114 ms / 200 ms | 17 ms / 42 ms |
| still screen, spinner line | 24-25 ms / 48 ms | 14-15 ms / 38 ms |

- Dark and light: colours, backgrounds, inverse, dim, italic, underline, bold, box drawing and the
  cursor as before. A Chinese row and an emoji row keep every part at its column.
- Dragging down shows earlier output.

## Not verified

- Pinch to resize: adb sends one pointer only. Left to the owner's check.
- Other phones. The cost scales with the phone's speed, and the dev phone is a slow one.
