# Verification: add-image-upload

Revision: `feat/image-upload` at 73e8167, on `main` d10dfeb.

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
Keyboard images were then removed (owner, 2026-11-09); the owner's own test of the picker found
the image key scrolled out of sight with the keyboard open, now pinned beside ⏎.

## On the phone by adb (2026-11-09)

TECNO CN7c, Android 16, debug build paired with an isolated test host
(`research/evidence/image-upload/2026-11-09-phone-check.md`). Passed: the image key; typing on
the moved field (erase, new line); a picked PNG to Claude Code (`[Image #1]` after the typed
text) and to Codex; a HEIC photo converted to a 1200x800 JPEG (`[Image #2]`); the host stopped
mid-send (no partial file, "The image didn't arrive", nothing pasted); closing a session deleted
its images.

## Not verified yet

- The image key with the keyboard open, by the owner (it was off screen in 73e8167; now pinned
  beside ⏎, seen there by adb).
- On screen, an isolated app instance paired with the test host: a PNG dropped and a screenshot
  pasted (once the Mac is free).
- agy with a pasted image path.
