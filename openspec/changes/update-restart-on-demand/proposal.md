# The update restarts only when the owner says

Status: plan, waiting for the owner's approval.

## Why

The owner (2026-10-31): restarting by itself once sessions are idle is not wanted. After Wait, the
owner should trigger the restart from an "Update Now" button, with a confirmation when sessions
are busy. Owner's choice: Wait still installs at once, so only the restart is left.

## Expected Outcome

- Update Now with busy sessions: the confirmation names them and offers Wait and Restart Now, as
  today.
- Wait installs the new version at once and never restarts by itself. The window then says it is
  installed; the sidebar's button reads "Update Now".
- "Update Now" in the sidebar: restarts at once when no session is busy; otherwise the confirmation
  names the busy sessions and offers Later and Restart Now.

## What Changes

- Updater: no automatic restart; the installed-and-waiting state waits for the owner (D1).
- Sidebar button "Update Now" for an installed update, with the confirmation (D2).
- `app-update` spec: busy sessions before a restart.

## Non-goals

- What counts as busy (working, needs you, background work, a plan to review: unchanged).
- Update Now with nothing busy (it installs and restarts at once, as today).

## Capabilities

No new project capability. Modified: `app-update`.

## Impact

`hq-app` (`workspace/update.rs`, `workspace.rs`). No host change.
