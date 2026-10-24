# Proposal

## Why

The owner wants a plain shell next to an agent without leaving Headquarter (2026-10-24):
Cmd-T opens a terminal under the shown session's pane, in that session's folder, with
the sidebar unchanged. Measured first (`research/notes/2026-10-24-session-shell-panel.md`):
an idle login zsh with the owner's setup costs 7.5 MB and no CPU; Headquarter itself adds
up to 1 MB of output buffer per open shell, and paints and holds a terminal model only
for the shell on screen.

## Expected Outcome

- Cmd-T (Ctrl-Shift-T on Linux) opens a shell panel under the shown session's agent, in
  the session's folder; Cmd-T again closes it and ends its shell.
- The panel stays open per session: switching sessions or restarting the app keeps the
  shell running, and coming back shows it again.
- Typing `exit`, or closing the session, ends the shell and closes the panel.
- Nothing runs for a session whose panel was never opened; shells are not agents (no
  status, notifications, history or usage).

## What Changes

- `hq-protocol`: requests to attach to a session's shell (starting it if needed) and to
  end it; the app remembers which panels are open, so session info is unchanged.
- `hq-daemon`: the host starts the owner's login shell in a PTY in the session's folder,
  keeps it in memory only (never in the index), ends it with the session, and leaves it
  out of status tracking and of the hosted processes the outside scan excludes, so an
  agent started inside the shell shows as running in another terminal.
- `hq-app`: the pane splits with a draggable divider (height remembered, default a third);
  Cmd-T toggles; focus moves to the shell when it opens and back to the agent when it
  closes; which sessions have an open panel is kept in `app-state.json`.
- `hq-app` (`shortcuts.rs`, `menus.rs`): the shortcut and a menu item.

## Decisions (owner, 2026-10-24)

Host-owned shell (rejected: app-owned, which every restart or update would end). Ends
when the panel is closed with Cmd-T (rejected: kept running hidden; ending after hours
hidden). Switching sessions or restarting the app does not close it. Closing the
session ends its shell at once (rejected: asking first).

## Non-goals

- Several shells or tabs per session; splitting sideways; a shell without a session.
- Restoring a shell after the host itself stops (it ends with the host, like agents).

## Capabilities

### New Capabilities
- `shell-panel`: a shell per session under its pane.

No project capability (in `AGENTS.md`) is added.

## Impact

`crates/hq-protocol`, `crates/hq-daemon` (host, session), `crates/hq-client`,
`crates/hq-app` (workspace pane, terminal view reuse, shortcuts, menus, app state),
tests.

## Approval

Approved by the owner on 2026-10-24 ("approve, go ahead").
