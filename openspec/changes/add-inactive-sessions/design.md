# Design

## Context

The index already stores a session whose agent is not running as `inactive`, with
`ended_at`, and marks the ones a host crash or reboot stopped with `interrupted_at`
(`recover_interrupted` at host start). The app shows every stopped session as idle
(`Glyph::of` maps `Inactive` to `Idle`) and restarts it when opened; the interrupted
ones are offered in the "N sessions were running" note. `on_exit` removes a session from
the sidebar when the owner closed it (✕) or the agent exited with code 0.

## Goals and non-goals

Goals: a quit keeps the session as inactive and visible; inactive sessions start only
when the owner asks; sessions that were running start again with the host. Non-goals:
automatic idle stopping, changes to the ✕ or to how agents are closed.

## Decisions

### D1. Three states for a stopped sidebar session, from existing fields

| State | Index | Shown |
| --- | --- | --- |
| Inactive | `inactive`, no `interrupted_at` | moon, dimmed, Resume Session pane |
| Starting | `inactive` with `interrupted_at`, or started by the queue and not ready yet | "starting" breathing where the age goes |
| Running | any other status | as today |

`interrupted_at` keeps its name and column; its meaning widens to "stopped with the
host, start it when the host starts". No new column is needed. Rejected: a new
`active` column (duplicates what the mark already says), renaming the column (churn in
a checksummed schema for no behaviour).

### D2. Exits

- ✕ (`closing`): leaves the sidebar, as today.
- Host stopping, for any reason (`hq stop`, a signal at logout or reboot): before ending
  agents, `shutdown` marks every live sidebar session `interrupted_at = now`, so the next
  host start resumes them (owner: "all active, even after hq stop"). A hard kill leaves
  them marked running, which `recover_interrupted` already handles.
- Any other exit (quit with 0, crash, signal): `end` only; the session stays in the
  sidebar, inactive, and its shell ends (owner: shell ends too).

### D3. Starting with the host

After `recover_interrupted`, the host queues sidebar sessions with `interrupted_at`, in
sidebar order, skipping any whose conversation is open in another terminal (the existing
"No resume while open elsewhere" check). One worker resumes them one at a time: the next
starts once the previous agent has reported itself (OSC 7501 or a hook), or after
5 s. A `Resume` from a client for a queued session runs at once and the worker skips it
when its turn comes; that is how the app starts the shown session first (D4). Each
resume clears the mark, as `reactivate` does today. Rejected: the app driving the queue
(sessions would not start when the host is started by `hq`), all at once (the owner chose
pacing).

### D4. The app

- `Glyph::Inactive` (a crescent moon, painted like the other glyphs, in the status
  glyph's place) for sidebar sessions not live, not marked, not running elsewhere; the
  row's title and age use the muted text colour.
- Every session the host will start shows "starting", breathing like "working", from
  the moment the host starts; each switches to its real status (usually idle) when its
  agent is ready, so the rows settle one by one in start order (owner, 2026-10-24).
  `SessionInfo.starting` carries it: true while queued, and after launch until the
  agent's first own report, OSC 7501 or a hook (the same moment that releases the next
  one). Plain output does not count: agents write terminal queries before they are up.
- The pane for an inactive session: agent mark, title, folder, "Inactive" (with the
  start failure reason when there is one) and a "Resume Session" button; Enter presses
  it while the pane has the keyboard. It replaces the "Open again" and "Closed in the
  other terminal." panes.
- On first list after connecting, when the shown session is marked and not live, the app
  sends `Resume` for it at once.
- ✕ on an inactive row: "Remove “title” from the sidebar?", then `Remove`.
- The interrupted note, `restart_interrupted` and `dismiss_interrupted` go.

### D5. Upgrade (migration `0008`)

Hosts before this change mark nothing on `hq stop`, so after the stop that installs this
version every session looks inactive. The migration marks sidebar sessions that ended
within 30 s of the most recent end among them (a host stop ends its sessions within
`STOP_GRACE`, 3 s), and clears older marks:

```sql
UPDATE sessions SET interrupted_at = CASE
    WHEN ended_at >= (SELECT MAX(ended_at) FROM sessions
                      WHERE sidebar_added_at IS NOT NULL AND status = 'inactive') - 30000
    THEN ended_at ELSE NULL END
WHERE sidebar_added_at IS NOT NULL AND status = 'inactive';
```

Run before `recover_interrupted`, so a reboot (sessions still marked running) is handled
by recovery as usual. Risk: a session quit within 30 s before that stop also starts once.
Rejected: starting all (the owner chose "not running → inactive"), none (would leave the
running ones inactive).

### D6. Start failures

A failed queued resume leaves the session inactive, clears its mark, and keeps the reason
in the host's memory (not the index); `SessionInfo.start_error` carries it with the time
it failed (`#[serde(default)]`). The app notifies for failures newer than the latest one
it has notified, which it keeps in `app-state.json`, so a failure is told once even when
the app was closed at the time or restarts (title: the session as the sidebar shows it;
body: "Couldn't start: <reason>"). The pane shows the reason; the host clears it on a
successful resume or removal. Rejected: persisting the reason (the index keeps status metadata only; a reason
that outlives the host would be stale).

## Protocol

`SessionInfo.start_error: Option<StartError { reason, at }>` and `SessionInfo.starting: bool` (both defaulted). `Request::Dismiss` is removed;
`PROTOCOL_VERSION` stays 9 (an older client sending it gets the usual bad-request error).

## Failure behaviour

- A queued session's folder or agent is gone: D6.
- The host stops while the queue runs: unstarted sessions keep their mark and start at
  the next host start.
- The app is not running when sessions start: they run; failures notify when the app next
  connects (the reason is still in the host).

## Rollback

Reverting restores quit-removes-from-sidebar; inactive rows then show as idle again, as
before. Migration `0008` only rewrites marks, which older versions read as the
interrupted note.

## Risks

- A burst at host start: each Claude session takes about 300 MB while starting; pacing
  spreads the CPU.
- An agent that exits at once on resume (for example a deleted conversation) becomes
  inactive with its exit code; the pane says so.
