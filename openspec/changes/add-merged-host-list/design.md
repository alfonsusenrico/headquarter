# Design: every host's sessions in one list

## Context

- The workspace holds one host's `sessions` and `groups`, filled by one `SessionFeed`;
  `restart_feed` clears them when another host is shown. Requests go through `host_call` to
  `hosts::current()`.
- Session ids are UUID v4 strings each host generates: unique across hosts. Group ids are SQLite
  integers per host: they collide.
- Step 1 keeps every paired host connected through one endpoint.

## Decisions

### D1. A session keeps its id; a map gives its host

`HostId` is `Local` or a paired host's endpoint id. The workspace keeps `session_hosts: id →
HostId` beside the merged `sessions`. Selection, panes, shell panels, details, notification tags
and `app-state.json` stay keyed by the session id, as today, so saved state needs no migration.

Rejected: a composite key (host, id) everywhere. Ids are already unique; the composite key would
touch every keyed place and the saved state for nothing.

### D2. One feed per host

The workspace opens a `SessionFeed` for this computer's host (when it has one) and each paired
host. Each feed's events carry its `HostId`. A host's list replaces only that host's sessions. A
feed that ends marks its host reconnecting: its sessions stay, greyed, until the feed returns.
`restart_feed` and the "host on screen" go; `hosts::current()` becomes `hosts::host(id)`.

### D3. Groups merged by name

The view: every host's groups, merged by name (case as first seen), in this computer's order, then
names only other hosts have, in their order. Each merged group gets a display id; the sidebar's
arrangement (`hq-protocol` `sidebar.rs`) runs on the merged groups and each session's group mapped
to its display id, unchanged. Actions:

- Move a session into a merged group: `SetGroup` on the session's host with that host's group of
  the name, made with `NewGroup` there first if it has none.
- New group: made on the dragged session's host, or this computer's when made empty.
- Rename, colour, collapse, delete, move: sent to every host holding the name.

Rejected: groups kept by the app across hosts. The phone shows each host's own groups and would
stop seeing groups made on the desktop (owner chose merge by name, 2026-11-09).

### D4. Every action goes to the session's host

`host_call` takes the session's id and looks up its host. Terminals attach through the session's
host. New Session and its folder browser take a host; Add sessions lists every host's past
conversations, each added on its own host.

### D5. Usage per host

Each host's usage is kept apart. The footer shows the selected session's host's usage (this
computer's when nothing is selected), with the host's name when it is not this computer. The Usage
window (CPU and memory, ⌘U) measures the same host and lists only its sessions, named "host on
<name>" when it is another computer.

Rejected: the Usage window listing every host at once. Each host is measured by its own request
every few seconds; one host at a time keeps the window as it is, and the selection picks it.

### D6. The Hosts menu

The header reads "Hosts" with "N hosts" beside it, and opens the list: each host with connected,
reconnecting or not reachable; Connect to a Host…; Forget on a paired host, which turns the row into
"Forget <host>? You'll need a new code from it to connect again." with Cancel and Forget of equal
width (Forget in the danger colour); Escape or a click elsewhere cancels.

### D7. Labels

A session's row shows its host's name when the host is not this computer. On a computer without a
host, every row shows it.

## Failure behaviour

- A host unreachable at start: its last known sessions do not exist yet in the app; the Hosts menu
  shows it not reachable, and its sessions appear when it connects.
- A host removed this computer: its sessions leave the list; the Hosts menu says to pair again.
- A group action fails on one host: the others still change; the notice names the host that
  failed.

## Risks

- Two hosts with groups of one name meant differently merge; the owner chose this.
- More feeds mean more host connections open at start; with a handful of hosts this is small.
