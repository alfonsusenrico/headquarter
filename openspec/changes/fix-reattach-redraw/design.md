# Design: a correct screen when coming back to a session

## Context

Attach today: `TerminalSession::connect` starts `run_connection`, which attaches at
`INITIAL_SIZE` (80 x 24). The host replays the catch-up buffer, then `redraw_at(80, 24)`
resizes the agent's terminal to 79 rows and back to 80 x 24. The view's first layout then sends
the real size. Measurements: `research/evidence/fix-reattach-redraw/findings.md`.

## Decisions

### D1. Attach at the pane's size

`TerminalSession` starts the connection on the first `set_size` from the view's layout, and
attaches with that size. A 500 ms timer attaches at 80 x 24 if no layout comes (a pane that is
never laid out still connects). The shell panel uses the same path.

Rejected: keep the placeholder and fix only the host. The agent would still draw an 80 x 24
frame into the catch-up buffer and the viewer on every attach.

### D2. A redraw the agent notices

`redraw_at` shrinks the terminal by one row, then restores it once the agent erases its screen
(`ESC [2J`, how the full repaint a size change asks for starts) or after 500 ms, whichever comes
first. A short-lived thread per redraw waits for that. Any other size change (a viewer's
resize, a viewer taking the session over, a newer redraw) cancels a pending restore, so the
final size is always the latest. A viewer's resize to the size it already has changes nothing,
so the app's resize right after attaching cannot cut a redraw short.

Measured: restoring at once gave no repaint, restoring 5 ms or more later gave two (the smaller
size and the restored one). Any output is not enough of a signal: a working agent writes
output it had queued before it handled the size change.

Rejected:
- A fixed delay alone: a busy agent can miss it.
- Any output as the signal: see above.
- Sending the agent a redraw key (Ctrl-L): agent-specific, and it reaches the agent as typed
  input.

## Risks

- The agent shows one row fewer for up to 500 ms after an attach. The viewer sees the agent's
  repaint at the smaller size, then at the full size.
- An agent that repaints without erasing the screen, or a plain shell, costs the full 500 ms
  before its size is restored. Nothing is drawn wrongly meanwhile.
