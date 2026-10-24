# Design

## Decisions

### D1. The host runs the shell

A shell is a PTY the host owns, like an agent session's, so it survives switching
sessions and app restarts. It is kept in memory only, keyed by its session, and never
written to the index; it ends with its session and with the host. Started as the owner's
login shell (`$SHELL -l`, the same shell the host resolves the owner's `PATH` from) in the
session's folder, `TERM=xterm-256color`.

### D2. Lifetime

Cmd-T opens it; Cmd-T while it is open hangs it up (as Kill does: hang-up, then killed
3 seconds later) and closes the panel. The shell exiting (`exit`, Ctrl-D) closes the panel.
Closing or removing the session hangs up its shell first. Switching sessions, hiding the
window and restarting the app leave it running; the app reattaches when the session is
shown again.

### D3. Not an agent

No status tracker, hooks, OSC handling for status, notifications, history import, usage or
sidebar row. The outside scan's exclusion list holds agents' processes only, so an agent
started inside a shell is seen as running in another terminal, not as the session's own
agent or its background work.

### D4. Layout and focus

The pane splits horizontally: agent above, shell below, with a draggable divider; the
shell's share is remembered in `app-state.json` (default one third; each part keeps at
least 6 rows). Opening focuses the shell; closing returns focus to the agent; a click
focuses either. The sidebar and the icon rail are unaffected.

### D5. Cost

Only the shown session's shell is attached in the app (one terminal model, one painted
view); other sessions' open shells cost only their own process and up to 1 MB of catch-up
buffer in the host, and no CPU while quiet. Both terminals painting busy output at once
is the heaviest case; the existing redraw cap applies to each view.

### D6. Keys

Cmd-T on macOS; Ctrl-Shift-T on Linux, because plain Ctrl keys go to agents (Claude Code
uses Ctrl-T) and shells. A Window menu item "Shell" shows it.

## Protocol

`AttachShell { session, cols, rows }` starts the session's shell if none runs and
attaches to it, answering `Attached` like `Attach`; `CloseShell { session }` hangs it up.
Input, resize and output then use the existing frames on that connection. `SessionInfo`
is unchanged: the app remembers which sessions have their panel open. Older hosts reject
the new requests with their usual error; `PROTOCOL_VERSION` stays 9.

## Risks

- Two terminals sizing one window: each PTY is sized to its own part of the pane.
- Shell startup time with heavy shell setups (0.3 s measured here).
