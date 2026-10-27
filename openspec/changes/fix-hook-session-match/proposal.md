# Status only from the session's own conversation

Status: awaiting the owner's approval.

## Why

The owner saw Headquarter dev marked "needs you" for minutes while it was working, with no
question on screen (2026-10-27, after 0.1.15 restarted and resumed the sessions). Findings:
`research/evidence/fix-hook-session-match/findings.md`.

Claude Code 2.1.311 starts a per-user background daemon from a running session, and the daemon
runs separate Claude Code sessions with the parent's command-line settings and environment. One
of them carried Headquarter's hooks and `HQ_SESSION` of Headquarter dev. The host files every
hook event and status-line report under `HQ_SESSION` and never checks the event's own Claude
conversation id, so that hidden session's "waiting" and permission events, and its context and
cost figures, land on Headquarter dev.

## Expected Outcome

- A Claude Code session's status, needs-you reason, and header figures come only from its own
  conversation. Events from another Claude Code conversation that carry the same `HQ_SESSION`
  are ignored and logged as such (ids only).
- `/clear` keeps working: the session adopts the new conversation Claude Code starts in place.
- A session resumed without an id (`--resume` with Claude Code's picker) adopts the first
  conversation it reports.

## What Changes

- `hq-daemon`: each hosted Claude session keeps its conversation id in memory (the id it was
  launched with). `hook_line` and the status-line path drop a Claude payload whose `session_id`
  differs, except a `SessionStart` with `source: "clear"`, which switches the session to the new
  id (and records it in the index, as a found conversation is today).
- The host log gains one line per ignored payload: `hook from another conversation id=<hq id>`,
  at most once a minute per session.

## Non-goals

- Codex and agy: their hook paths are not affected by Claude Code's daemon.
- Stopping Claude Code from inheriting the environment: the daemon is Claude Code's own design.

## Capabilities affected

`agent-status`. No new project capability.

## Impact

Host only, `crates/hq-daemon` (and a test in `crates/hq-cli/tests/host.rs`). No protocol change.
