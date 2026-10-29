# Working sessions first in their group

Status: approved by the owner 2026-10-29 (chosen in the approval question after "let's continue our
work to finish the fixes planned"; started on "merged 90, start working sessions first").

## Why

The owner's finding (2026-10-28, screenshot): "cash flow tracker dev" had been working for about
24 minutes but sat second in "Personal project", below "Headquarter dev", which had finished
17 minutes earlier. Working is live, so the owner wants working sessions at the top.

Cause, from the code: `layout` in `hq-protocol` `sidebar.rs` orders every list by
`last_activity_at`, newest first. The host moves that time only when the status changes
(`refresh` in `hq-daemon` `host/mod.rs`), so a working session keeps the time its turn started and
sinks below any session that finished after that.

## Expected Outcome

- Inside each group, and in the ungrouped list, sessions needing the owner come first, then
  working sessions, then the rest. Each band is ordered by most recent activity.
- Groups keep the owner's order. The "Needs you" section is unchanged.
- While the pointer is over the sidebar (or the icon rail), or a session or group is being
  dragged, rows hold their places and only their status changes. When the pointer leaves, they
  move to their new places at once.
- `hq ls` and the phone's home show the same order, since they share `layout`. The phone holds
  its rows while a finger is on the list.

## What Changes

- `hq-protocol` `sidebar.rs`: `layout` orders each group and the ungrouped list by band, then by
  recent activity.
- `hq-app` sidebar: holds the shown order while the pointer is over the sidebar or rail, or a drag
  is under way.
- Android home: holds its rows while a finger is on the list.
- Specs: `session-sidebar` (sections and order) and `android-app` (holding the order on touch).

## Non-goals

- Background work (subagents, shells, monitors) counting as working: decided later with
  `add-background-work-status` (owner, 2026-10-28).
- Moving groups that hold a working session above the others: the owner chose to keep the
  owner's group order.
- A separate "Working" section, and animated row moves.

## Capabilities affected

`session-sidebar`, `android-app`. No new project capability.

## Impact

Desktop app, `hq ls`, and the phone app. No change to the host, the index, or the protocol: the
order is computed from fields every client already receives.
