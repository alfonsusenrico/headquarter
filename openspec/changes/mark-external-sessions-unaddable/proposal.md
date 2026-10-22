# Proposal

## Why

The owner asked on 2026-10-22 that sessions running in another terminal cannot be
picked in Add sessions: that terminal owns them, so adding them here has no use. Until
now such a session could be picked to watch it, listed first under "Running in
another terminal".

## Expected Outcome

- In Add sessions, a session running in another terminal stays in date order, greyed,
  tagged "In another terminal", and cannot be picked.

## What Changes

- `hq-app` (`modal/open_sessions.rs`):
  - no "Running in another terminal" section; those sessions sort by date with the
    rest;
  - they are tagged "In another terminal" and cannot be picked;
  - the row's second line is always the folder;
  - the green "running elsewhere" dot and its colour argument are removed.

Decisions (owner, 2026-10-22): **greyed in place** (A of three options in the local
preview). Rejected: their own section at the end (B), and hiding them with a count (C).

## Non-goals

- Sessions already in the sidebar that run elsewhere: they stay observed as before.

## Capabilities

### Modified Capabilities
- `session-sidebar`: Add sessions does not offer sessions running in another terminal.

## Impact

`crates/hq-app/src/modal/open_sessions.rs`, `workspace.rs`, and their tests.

## Approval

Approved by the owner on 2026-10-22 ("4. A").
