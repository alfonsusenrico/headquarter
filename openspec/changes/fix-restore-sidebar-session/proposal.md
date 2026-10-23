# Proposal

## Why

The owner opened the app with an empty sidebar, and the pane showed a
session that was not in the sidebar (owner
screenshot, 2026-10-22). The app keeps the id of the session it shows, saved when a
session is opened. The owner had added that session, its resume was refused because
it ran in another terminal, and the owner closed it. Closing took it out of the
sidebar, but the kept id stayed, and the next launch showed it again.

## Expected Outcome

- After a relaunch, the pane shows the kept session only while it is still in the
  sidebar; otherwise it shows nothing in particular.

## What Changes

- The app restores the kept session only if it is in the sidebar
  (`sidebar::model::kept_session`).

Decisions:
- **Check at restore.** A session can also leave the sidebar while the app is closed
  (`hq remove`), so only the restore can know. Rejected: clearing the kept id when
  the pane empties (misses removals made while the app is closed).

## Non-goals

- What the pane shows while the app runs is unchanged.

## Capabilities affected

- session-restore: the shown session comes back only while it is in the sidebar.

## Impact

- `crates/hq-app/src/sidebar/model.rs`, `crates/hq-app/src/workspace.rs`. Adds no
  project capability.
