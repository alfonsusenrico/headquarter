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

`redraw_at` shrinks the terminal by one row, then restores it once the agent has written output
after the shrink (it handled the size) or after 500 ms, whichever comes first. It runs on a
short-lived thread per session. A newer redraw for the same session cancels a pending restore
and starts again, so the final size is always the latest viewer's.

Measured: restoring at once gave no repaint, restoring 5 ms or more later gave two (the smaller
size and the restored one). Waiting for the agent's output rather than a fixed delay covers a
busy agent that handles signals late.

Rejected:
- A fixed delay alone: a busy agent can miss it.
- Sending the agent a redraw key (Ctrl-L): agent-specific, and it reaches the agent as typed
  input.

## Risks

- The agent shows one row fewer for up to 500 ms after an attach. The viewer sees the agent's
  repaint at the smaller size, then at the full size.
- An agent that writes nothing after a resize (a plain shell prompt) costs the full 500 ms
  before its size is restored. Nothing is drawn wrongly meanwhile.
