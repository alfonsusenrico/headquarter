# Design: the update log reads from the top and follows the update

## Context

`render_log` draws the last `LOG_LINES` (6) of `updates.log` in a 132 px box with
`justify_end` and `overflow_hidden`, the newest line in the text colour and the others muted. The
log grows a line at a time from the installer's progress (`Progress(Step, Vec<LogLine>)`).

## Decisions

### D1. A top-aligned list that scrolls

The same 132 px box, now a vertical scroll area with the window's scrollbar, its lines starting
at the top. It holds every line of the current update (an update writes a few dozen), so
`LOG_LINES` goes.

### D2. Following the newest line

The box keeps a scroll handle. After a line is added, it scrolls to the bottom when the owner was
already at the bottom, or had not scrolled yet. While the owner has scrolled back, the view stays
where they put it. Scrolling back to the bottom resumes following.

Rejected: always jumping to the bottom (the owner could not read back during a slow download),
and a taller box (the window would grow with every line).

## Risks

- GPUI's scroll offset updates after layout, so "at the bottom" is read from the last frame's
  offset and content size. A line arriving in the same frame as the owner's scroll may follow once
  more than expected.
