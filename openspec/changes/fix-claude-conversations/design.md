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

Decided by experiment (task 1.3): `claude stop <background id>`, where the background id is the
`id` field of `claude agents --json` (`b0981cf4`), not the conversation id. `claude stop` with the
conversation id answers "No job matching", which is what failed for the owner. After the stop,
the host waits until Claude Code no longer lists the conversation (up to 8 s), then resumes it.
`claude stop` runs with a 10 s limit.

Also, when a Claude session's agent exits (not while the host stops), the host checks once
whether the conversation is still running in the background, and logs it (ids only), so a
refused resume is not the first sign. The outside scan's entry carries a `background` flag
(protocol `External`), so the app tags such a session "In Claude Code's background" and its
pane offers Bring into Headquarter instead of "close it there". The direct check runs before
the scan-based refusal, which stays as a fallback, and is asked only for a conversation with a
saved transcript, reusing one answer for 3 s.

### D4. One entry per chain

Add sessions groups conversations linked by `continued-in` records and shows the newest one,
with the chain's latest activity. Opening it opens the newest. Older links stay in the index,
and are no longer offered.

### D5. Logging adds and removes

Every change to sidebar membership logs a line with the row ids: `sidebar added ids=…` and
`sidebar removed ids=…` (Remove and Close both pass through it), and merges log which rows they
kept or removed. No titles. The host cannot tell a phone's request from the app's (the gateway
passes phones in as ordinary clients), so the lines carry ids only. This finds what removed the
owner's row three times. Task 2.3 tries to reproduce it.

## Risks

- `claude agents --json` takes about a second. It runs once per resume of a Claude session,
  not on every start of the app.
- The transcript format is an unstable external contract: parsed defensively, versions recorded.
