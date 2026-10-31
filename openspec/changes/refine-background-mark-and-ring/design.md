# Design: background's mark, and a ring that stays red

## D1. Stacked layers

A painted glyph on the 24-unit grid, in `chrome.teal`: a back sheet outlined at 60 % opacity and a
front sheet filled, as on the options page. It takes the glyph cell like review's diamond, so the
row's word ends where the other words end and the dots go. Static.

## D2. Near limit stays red when old

When a reading is old (the "Couldn't update since" note), a ring at or above `NEAR_LIMIT_PERCENT`
uses `near_limit()` at 55 % opacity instead of `chrome.muted`; a ring below it stays muted. The
non-text contrast test covers the softened red against the sidebar and the track at 3:1.

Rejected: keeping the full red (a reading that may be stale would look as sure as a fresh one).
