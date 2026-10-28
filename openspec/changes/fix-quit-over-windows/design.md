# Design: Quit's question over every window

## Context

`Workspace::render` adds its overlays as children in order, and a later child paints over an
earlier one: the modal windows, the confirmations, `render_quit`, then `render_update_window`,
`render_remote_window` and `render_resources`. `request_quit` focuses the question's own handle,
and `cancel_quit` only clears the state.

## Decisions

### D1. The question is drawn last

`render_quit` moves to the end of the overlays, with a comment saying why it must stay last.
Quitting is the one question that can arrive from anywhere (⌘Q, the menu, the window's close
button), so it must cover whatever is open.

Rejected: closing the other windows when Quit is asked. Cancel would then lose what the owner
had open.

### D2. Focus returns on Cancel

`request_quit` keeps the focused handle before it takes focus, and Cancel (button or Esc)
focuses it again when it still exists. Quit and Keep Running end the app, so they need nothing.

## Risks

None known. No test reaches the workspace's drawing order (the workspace needs a host), so the
fix is checked on screen.
