# Verification

Working tree on `fix/update-log-scroll` (rebased onto 49826f1), 2026-10-29, macOS.

## Automated

- `./scripts/check`: exit 0. 546 Rust tests passed, 0 failed (1 ignored, as before); rustfmt,
  clippy `-D warnings`, cargo-deny; OpenSpec 19 of 19 valid (strict); Android ktlint, Lint,
  unit tests and builds.
- New test `the_log_follows_only_from_its_end`: following while not scrolling yet, at the end and
  within the slack; not while scrolled back or at the top.

## On screen

The dev app with a synthetic `HOME` (an install manifest naming the dev app, so it can update) and
`HQ_UPDATE_BASE` pointing at a local GitHub stand-in (`research/prototypes/update-standin`,
release v0.1.99) whose installer prints progress and notes in the real format, slowly, then stops
before installing anything.

- Check for Updates found v0.1.99; Update Now opened the window. The first lines started at the
  top of the box, with space below them.
- Once the lines filled the box, it scrolled by itself, the newest line at the bottom and
  brightest, with a scrollbar.
- Scrolled back during the update: the view held for 9 s while new lines arrived.
- Scrolled back to the end: the newest line showed, and 9 s later the box had followed to the
  next lines ("Downloading part 14 of 14").
- The failed state ("The update did not finish") keeps the same scrolling log.

## Not verified

- A real update to a published release (the next release will be the first).
- Linux.
