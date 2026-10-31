# Verification: fix-false-done

Working tree on `fix/false-done` (committed with this record), 2026-10-31.

## Checks

- `./scripts/check`: passed. 572 tests, clippy `-D warnings`, rustfmt, cargo-deny, Android build,
  ktlint, Android Lint, OpenSpec (19 items).
- New and changed tests:
  - `hq-app` notifications: a reply is announced only after 10 quiet seconds; working again,
    reading it or needing the owner drops it; background work holds it until 10 s after the work
    ends; replies turned off, or the first listing, never wait; needs-you stays immediate; the
    settled notice keeps the sidebar's title, the session's tag, and the watched and
    sidebar rules.
  - `hq-app` update: background and review sessions are busy, stopped ones are not.
  - `hq-mobile` mirror: an unread reply with background work stays in its group showing
    "background" and rises once the work ends.

## Not verified

- Notifications on screen: macOS shows them only for the installed, signed app, so checking the
  10-second hold needs a local build installed (the owner's go) or the next release.
- The update window waiting on a background session on screen (needs the update stand-in and the
  owner's go for the Mac).
- The phone's top section on a device.
