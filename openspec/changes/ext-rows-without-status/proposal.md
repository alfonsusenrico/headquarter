# Proposal

## Why

Using the new "add a session running elsewhere" (owner, 2026-10-23): a sidebar session
still running in another terminal showed that terminal's status ("working", "needs
you"). That terminal owns it; the owner wants only the "ext" badge until the session
runs inside Headquarter, and its status then.

## Expected Outcome

- A sidebar session running in another terminal shows its mark, title, "ext" and age:
  no status word, no status glyph, no "Needs you" listing, no notification.
- Once it runs in Headquarter, its status shows as for any other session.

## What Changes

- `hq-protocol` (`sidebar.rs`): `needs_you` counts only sessions not running in another
  terminal, so the app's "Needs you", group counts, the icon rail and `hq ls` leave
  them out.
- `hq-app` (`sidebar/model.rs`, `sidebar/glyph.rs`): such a row draws no status glyph
  and no status word; the slot stays, so ages line up.
- `hq-app` (`notifications.rs`): no notification for such a session.

Decisions (owner, 2026-10-23): badge and age only (rejected: badge only; badge with
needs-you kept).

## Non-goals

- `hq ls --all` keeps printing the outside status in its status column; the host keeps
  reading it (the pane note and usage cadence use it).
- No change to Add sessions beyond its "Added while running elsewhere" wording.

## Capabilities

### Modified Capabilities
- `session-sidebar`: rows of sessions running elsewhere show no status.
- `notifications`: none for sessions running elsewhere.

No project capability is added.

## Impact

`crates/hq-protocol/src/sidebar.rs`, `crates/hq-app/src/sidebar/model.rs`,
`crates/hq-app/src/sidebar/glyph.rs`, `crates/hq-app/src/notifications.rs`, tests.

## Approval

Approved by the owner on 2026-10-23 ("Badge and age only").
