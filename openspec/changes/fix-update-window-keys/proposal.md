# The update window takes the keyboard

Status: requested by the owner on 2026-10-28 ("fix the update notice Esc"), which approves this
fix.

## Why

Found while checking fix-quit-over-windows (2026-10-28): Check for Updates' notice
("Headquarter 0.1.16 is the latest version.") never closes on Escape. The update window has no
focus of its own, so the keyboard stays with what was under it, usually the session's terminal.
Escape and anything typed while the window is open go to the agent behind it.

## Expected Outcome

- While the update window is open it holds the keyboard: nothing typed reaches the session
  behind it.
- Escape closes it wherever it offers OK, Later or Close: the latest-version notice, the offer,
  "Restart to finish updating", a failed update, and the wait after installing.
- While an update installs or restarts, and while it asks about busy sessions, Escape does
  nothing, as those have no way to dismiss them today.
- Closing the window puts focus back where it was.

## What Changes

- `hq-app` `workspace/update.rs`: the window's focus, Escape, and giving focus back.
- `hq-app` `workspace.rs`: the focus handle and the focus to return to.
- `app-update` spec: a requirement for the window's keys.

## Non-goals

- What the window offers, says or does.
- The What's new page (it already closes on Escape).

## Capabilities affected

`app-update`. No new project capability.

## Impact

Desktop app only. No agent CLI is involved.
