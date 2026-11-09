# Verification: add-image-upload

Revision: `feat/image-upload`, on `main` d10dfeb. On-phone and on-screen checks (task 4.3) wait for
the owner's go.

## Automated (2026-11-09, macOS, JDK 17)

| Check | Result |
| --- | --- |
| `./scripts/check` (fmt, clippy `-D warnings`, tests, cargo-deny, OpenSpec) | passed: 625 tests, OpenSpec 19/19 |
| `hq-protocol`: `Upload` round trip, `ImageKind` by first bytes, paste and typed paths | passed |
| `hq-daemon` `host/images.rs`: saved owner-only, over 10 MiB refused before writing, a PDF refused and removed, cut short leaves nothing, more bytes than announced, the sweep (a week, a session gone, stale partial files, a foreign file kept) | passed |
| Host test, raw socket: a 600 kB PNG in 3 frames saved with modes 600/700 and its path answered; 10 MiB + 1 refused (`TooLarge`); a PDF refused; an unknown session refused; cut short by the connection and by another request leave nothing; the log has session, bytes, kind and no path; closing the session deletes its folder | passed |
| Host test: images 8 days old and of a session no longer in the sidebar are deleted at start; a 1-day-old one stays | passed |
| Host test, desktop over a remote link through the gateway: a 700 kB PNG uploaded with progress in steps, the path names the host's file | passed |
| Host test, phone core through the gateway: a PNG sent on a second stream while the session stays open; saved, and its path reaches the session | passed |
| `hq-app` `terminal/images.rs`: a clipboard PNG sent as it is, a bitmap and a TIFF converted to PNG, an SVG refused; a dropped file judged by its bytes, over 10 MiB, a folder | passed |
| `./gradlew ktlintCheck assembleDebug lint testDebugUnitTest` | passed: 17 JVM tests (`SendImageTest`: formats, conversion rule, scaling), Lint clean |

Compose's keyboard content check: `research/evidence/image-upload/2026-11-09-content-receiver.md`.

## Not verified yet

- On the owner's phone with a test host: a picked screenshot to Claude Code and Codex
  (`[Image #1]`), a HEIC photo, a keyboard screenshot, the link dropped mid-send, typing still
  live after the field moved to `TextFieldState`.
- On screen, an isolated app instance paired with the test host: a PNG dropped and a screenshot
  pasted.
- agy with a pasted image path.
