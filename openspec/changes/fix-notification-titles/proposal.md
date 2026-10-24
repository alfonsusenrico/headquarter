# Proposal

## Why

Notifications were titled with the session's name (add-notifications D2, 2026-10-22:
never the agent's own title), so a session the owner never renamed showed its id,
such as "claude-e0c6f139", and the owner could not tell which session replied
(2026-10-24).

## Expected Outcome

- A notification is titled with what the sidebar row shows: the owner's name for the
  session, else the agent's title, else "Untitled conversation · id".

## What Changes

- `hq-app` (`notifications.rs`): the title comes from `sidebar::display_title`, as the
  sidebar's.

Decision (owner, 2026-10-24): the sidebar's title, reversing D2's "never the agent's own
title". macOS keeps notifications in Notification Center's history, so an agent's title
is stored there; Headquarter itself still stores no titles (index, log and evidence
files stay title-free, as `AGENTS.md` says). Rejected: keeping the name.

## Non-goals

- The body ("Claude replied", "needs your permission") and clicks are unchanged.

## Capabilities

### Modified Capabilities
- `notifications`: titled as the sidebar row.

No project capability is added.

## Impact

`crates/hq-app/src/notifications.rs` and its tests.

## Approval

Approved by the owner on 2026-10-24 ("Sidebar title").
