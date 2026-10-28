# Design

## Context

`layout` (`hq-protocol` `sidebar.rs`) is shared by the app's sidebar and icon rail, `hq ls`, and
the phone's home (`hq-mobile` `mirror.rs`). It sorts sidebar sessions by `last_activity_at`,
newest first, then splits them into groups and the ungrouped list. A status change that counts as
activity (`is_activity` in `hq-daemon` `session.rs`) sets that time. A working session keeps the
time its turn started.

Owner decisions (2026-10-28): working sessions go to the top of their own group, and groups keep
the owner's order. Sessions needing the owner go above working ones. Rows hold still while the
owner points at the sidebar. Background work waits for `add-background-work-status`.

## Decisions

### D1. Three bands per list, recent activity inside each

Each group's list and the ungrouped list are ordered by band: needing the owner (`needs_you`), then
working (`Status::Working`, not running elsewhere), then everything else. Inside a band, the order
is most recent activity first, then by id, as today.

A session running in another terminal shows no status (`ext`), so it is always in the last band.
A starting session is in the last band, and its fresh start time already puts it near the top of
that band.

Rejected: one sort key that treats working as "active now". It gives the same result for working
sessions but hides the rule. Rejected: ordering the working band by how long each has been
working. Newest first keeps one rule for every band, and a session that has just started a turn is
the one the owner most likely just spoke to.

### D2. Hold the shown order while the owner is using the sidebar

The app keeps the row order it last showed. While the pointer is over the sidebar or the icon rail,
or a session or group is being dragged, it shows the new layout in the held order:

- a session in the held order keeps its place relative to the others in the same list;
- a session not in it (a new session, or one the owner just moved to another group) takes its
  place from the new layout.

Statuses, words, glyphs and ages still update live. Only positions hold. When the pointer leaves
and no drag is under way, the new order applies at once. Menus, renames and keyboard selection act
on the session id, so a move never changes their target. Keyboard navigation follows the shown
order.

This is a pure function in the sidebar model (held order + new layout → shown order), unit-tested
on its own.

Rejected: reorder immediately (a row can move just before a click; owner, 2026-10-28). Rejected: a
fixed delay before moving (the row can still move under the pointer once the delay ends).

### D3. The phone holds its rows while touched

The phone has no pointer, so its home holds the shown order from when a finger touches the list
until it lifts and any fling stops, with the same rule as D2. The order comes from the host's
`layout`, so it needs no ordering code of its own.

## Risks

- Sessions that start and stop turns often move often. Mitigation: holding while the pointer is
  on the sidebar, and only working and needs-you sessions rise.
- The phone's hold adds touch state to the home list. If it proves fragile on screen, it can ship
  without the hold, as a follow-up with the owner's agreement.

## Open questions

- None for this change. Whether background work counts as working is decided in
  `add-background-work-status`.
