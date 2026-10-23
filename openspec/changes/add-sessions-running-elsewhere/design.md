# Design

## Context

Add sessions builds its rows from the host's list. A row whose session has
`external` set is tagged "In another terminal" and blocked (`blocked =
Some(IN_ANOTHER_TERMINAL)`). The host's add request refuses only outside agents with no
known conversation (`host/mod.rs`), so `hq add` already accepts a linked one. In the
sidebar, a session with `external` set shows "ext"; opening it shows the details pane
"Running in another terminal (process N)…" and starts nothing (`Workspace::open`).
When the outside process ends, the outside scan clears `external` within about 5
seconds and the pane falls through to "<agent> stopped." with "Open again".

## Decisions

### D1. Pickable only when the conversation is known

A row is blocked only when it is an outside agent with no linked conversation. A
linked outside session is pickable and keeps the tag, in date order, drawn like other
rows. Rejected: allowing unlinked agents (nothing to continue later; the host refuses
them anyway).

### D2. The pane remembers that the other terminal closed it

The workspace keeps the id of a shown session it last saw running elsewhere. When an
update clears `external` for that session while it is shown and not live, the pane
says "Closed in the other terminal." with an "Open here" button that resumes it, as
"Open again" does. Choosing another session, or opening it, forgets the id. Rejected:
resuming at once (owner chose to decide each time), and reusing "<agent> stopped."
(it reads as a failure in Headquarter).

### D3. No protocol change

The list already carries `external` and its process id. Adding uses the existing add
request. Nothing new is stored.

### D4. Unlinked outside agents are kept current in clients

The host lists unlinked outside agents (`Origin::External`, id
`external-<agent>-<pid>`) in its answer to a list request, but sent no change when one
appeared, linked or exited, so a client kept its first view. On screen, the dev host's
first scan ran before its first history import: every Claude process was unlinked,
linked seconds later, and the greyed rows stayed beside the linked ones.

Each outside scan now compares its unlinked entries with the previous scan's: a new or
changed entry is sent as a session change; one that is gone (linked or exited) is sent
once more with `offered = false`, which clients already treat as "do not offer"
(remove-conversations). Rejected: a new "session removed" message (a protocol change
for what `offered` already says), and re-listing in the app on a timer (more traffic,
and still stale between lists).

### D5. Rows line up

A row that cannot be picked keeps an empty slot the tick's width, so the tag and age
columns line up with pickable rows.

## Failure behaviour

- The other terminal still runs it when the owner chooses "Open here" (closed and
  reopened in between): the host's existing refusal names the process, and the pane
  shows that reason with "Open again".
- The outside scan misses the exit: the pane keeps saying it runs elsewhere until the
  next scan, as today.

## Risks

- Two quick toggles (closed, then reopened outside before the scan) can show a stale
  pane for up to one scan interval; the host still refuses to resume while it runs.
