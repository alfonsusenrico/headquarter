# Proposal

## Why

The owner opened the app with four Claude Code sessions running in other terminals.
The two that were working showed "Running in another terminal"; the two idle ones
looked addable. Adding one failed with "running in another terminal", so the host knew
the session ran elsewhere and the app did not (owner report, 2026-10-22).

Cause: the app's connection lists the sessions, reads usage, and only then subscribes
to changes. The host tells only subscribers about a change. The outside scan, woken by
the list, linked the idle sessions while the app waited for the usage answer: the
Codex reading made that wait about 3 s (host log, 2026-10-22). That change reached
nobody. Working sessions change status again and again, so a later change corrected
them; idle ones never change, so the app kept the stale row until something else
changed it.

## Expected Outcome

- When the app opens, every session running in another terminal is shown as such,
  idle or working, as soon as the host knows it.
- No change the host reports while the app's connection opens is lost.

## What Changes

- `hq-client` `SessionFeed::open` subscribes first, then lists and reads usage.
- Changes that arrive before an answer are kept and reported first, in order.

Decisions:
- **Subscribe before the list (client).** The gap is in the client's opening order,
  so the fix belongs there. Rejected: answering `Subscribe` with a fresh list (a
  protocol change for every client). Rejected: only moving `Subscribe` ahead of
  `Usage` (narrows the gap to milliseconds without closing it).

## Non-goals

- The usage answer still waits for a Codex reading when one is due; that delays the
  app's first list but loses nothing once the feed subscribes first.

## Capabilities affected

- external-sessions: clients learn every link made while they connect.

## Impact

- `crates/hq-client/src/lib.rs` only. No protocol, index, or settings change. Adds no
  project capability.
