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

## On screen (the owner freed the Mac, 2026-10-31)

Sandbox (`/private/tmp/hqu`: synthetic `HOME` whose install manifest lists the dev build, stand-in
agents from `research/prototypes/background-status`, the update stand-in serving v0.1.99 with
`STANDIN_SUCCEED=1` so its installer reports success without installing). Sessions: "api
migration" (Claude, a background agent running), "release plan" (agy, a plan to review), "notes"
(idle). Captures in `research/evidence/false-done/`:

- The sidebar shows "review ◆" and "background ••". Update Now: "2 sessions are busy: api
  migration, release plan" (`update-asks-background-review.png`); "notes" is not listed.
- Wait: the stand-in install finished, then "Waiting for 2 busy sessions before restarting"
  (`update-waits-for-2.png`); 0.1.20 would have restarted here.
- The background agent ends: "Waiting for 1 busy session"; "api migration" shows its age again
  (`update-waits-for-review.png`).
- The app was quit before the review cleared: the restart reopens the app outside the sandbox.
  Restarting once nothing is busy is unchanged behaviour (app-update spec, earlier checks).

## Not verified

- Notifications on screen: macOS shows them only for the installed, signed app, so checking the
  10-second hold needs a local build installed (the owner's go) or the next release.
- The phone's top section on a device.
