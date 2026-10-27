# Design: keep Claude Code sessions on their own conversation

## Context

Claude Code 2.1.311: conversations can run in a per-user daemon (`claude bg-spare` processes,
`kind: background`). A resumed conversation can continue under a new id, recorded only as a
`continued-in` line in the old transcript. One conversation can be continued twice: 43096428
went to 582a1c38 at 13:51 and to 64d62cfc at 15:47, the second from the stale resume. Findings
in `research/evidence/claude-daemon-sessions/` and `research/evidence/claude-continued-conversation/`.

## Decisions

### D1. Follow the continuation while the session runs

After a Claude session starts, and on every completed turn, the host checks the session's
current transcript for a `continued-in` record and adopts the newest id it names (the record's
`continuedInSessionId`), recording it in the index. A report from that id is the session's own
(`fix-hook-session-match` treats it as Own). So the stored id is current before any restart.

Rejected: following the last `continued-in` record only at resume time (the urgent fix). When
a conversation was continued twice, the last record can name the wrong branch, as 64d62cfc did.
At resume it stays a fallback for a session that never got to record its continuation: it then
takes the newest continuation by the transcript's last write time, not the last record.

### D2. Merges never delete the owner's rows

When two rows end up on one conversation, the kept row is the one in the sidebar, else the one
the owner named, else the hosted one. A row is deleted only when it is imported from history,
outside the sidebar, and not named by the owner. If both are in the sidebar or both named, the
newer id is kept on the older row and nothing is deleted. A test covers the 2026-10-27 case.

### D3. Background sessions

Before starting `claude --resume <id>`, the host asks Claude Code which conversations run
(`claude agents --json`, which the outside scan already reads). If `<id>` runs with
`kind: background`, the resume returns `Response::Error` with a new kind `RunningInBackground
{ pid }` (protocol 12) instead of starting an agent that exits at once. The app shows on the
session pane: "This conversation is running in Claude Code's background (pid N)" with "Bring
into Headquarter". That action stops the background session, then resumes it.

How to stop it is decided in task 1.3 by experiment. Candidates: `claude stop <id>` (did not
work for the owner), the `/stop` command sent to an attached front end, or ending the spare
process. The design records the one that works.

Also, after the host ends a Claude session (close, stop), it checks once whether the
conversation is still running in the background, and logs it (ids only), so a restart is not
the first sign.

### D4. One entry per chain

Add sessions groups conversations linked by `continued-in` records and shows the newest one,
with the chain's latest activity. Opening it opens the newest. Older links stay in the index,
and are no longer offered.

### D5. Logging adds and removes

`Add`, `Remove`, `Close`, and any merge that changes a row's sidebar membership log a line with
the row ids and the client kind (app, CLI, phone). No titles. This finds what removed the
owner's row three times. Task 2.3 tries to reproduce it.

## Risks

- `claude agents --json` takes about a second. It runs once per resume of a Claude session,
  not on every start of the app.
- The transcript format is an unstable external contract: parsed defensively, versions recorded.
