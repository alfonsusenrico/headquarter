# Design: the update window takes the keyboard

## Context

`render_update_window` draws the window while `updates.open` is set. Five places set it (a
manual check that finds nothing or fails, the update pill when no page applies, What's new's
Update Now, and the restart), and `close_update_window` clears it. The window tracks no focus,
so keys go to whatever had focus. The quit dialog and Remote Control each track their own handle.

## Decisions

### D1. One way to open, one way to close

`open_update_window(window, cx)` sets `open`, remembers what has focus when the window is not
already open, and focuses the window's handle. Every place that opened it calls this.
`close_update_window(window, cx)` clears it and focuses what it remembered. `show_whats_new`,
which swaps the window for the page, drops the remembered focus, since the page takes it.

### D2. Escape follows the dismiss button

Escape closes the window in the phases that show OK, Later or Close: `Offer`, `Waiting` and
`Failed`. In `Asking`, `Running` and `Restarting` it does nothing, matching their buttons. A
small function decides this, and a unit test covers each phase.

## Risks

- Focus remembered from a view that closed meanwhile (a session removed while the window was
  open) is a handle that no longer exists in the window, and focusing it does nothing, the same
  as Cancel in the quit dialog.
