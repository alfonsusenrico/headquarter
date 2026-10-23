# Proposal

## Why

Add sessions still offered conversations the owner never had or can no longer continue
(owner report, 2026-10-23): untitled Claude rows in one project folder whose
transcripts Claude Code had deleted (it removes them after about 30 days by default),
runs a program started (`claude -p` or the Agent SDK, `codex exec`), and agy
conversations opened and left before a single step. The owner asked that the same
rule hold for every agent, not only agy.

Sub-agents were already kept out for Claude Code (stored apart) and agy (marked with a
parent), and for Codex only by accident: a sub-agent's rollout carried its parent's id
and folded into the parent's row.

## Expected Outcome

- Add sessions offers, for every agent, only conversations the owner started that the
  agent still keeps and whose folder exists.
- Nothing is deleted: the agents' own history and Headquarter's index are untouched,
  `hq ls --all` lists every row, and sessions in the sidebar stay.

## What Changes

- `hq-signals` (`history.rs`): each conversation says whether the owner started it:
  Claude Code's `entrypoint` (`sdk-…` is a program) and whether its transcript holds a
  prompt or reply at all (one without either was opened and left, or is the stub Claude
  Code leaves with a `continued-in` record when a conversation goes on under a new id;
  12 of the owner's 43 transcripts), Codex's `source` (`exec` is a
  program; a `{"subagent": …}` source is a sub-agent and is not a conversation of its
  own), agy's `step_count` (0 is never used).
- `hq-daemon`: each history pass keeps, in memory, the past conversations to offer: those
  found in the agents' history that the owner started. Rows for anything else are
  reported with `offered: false`, and clients hear when that changes.
- `hq-protocol`: `SessionInfo.offered`, which an older host's missing value reads as true.
- `hq-app`: Add sessions keeps only offered sessions (with fix B's folder check).

Decisions (owner, 2026-10-23): one rule for every agent; mark rather than delete.
Rejected: deleting index rows (would hide them from `hq ls --all` too), filtering by
folder name or title (no fixed pattern).

## Non-goals

- Sub-agents in the sidebar, or listing them anywhere.

## Capabilities

### Modified Capabilities
- `session-history`: which past conversations are offered.
- `session-sidebar`: Add sessions keeps only offered conversations.

## Impact

`crates/hq-signals/src/history.rs`, `crates/hq-daemon/src/host/{history,mod,outside}.rs`,
`crates/hq-protocol/src/lib.rs`, `crates/hq-app/src/modal/open_sessions.rs`, and the
`SessionInfo` literals in tests. No index change. Adds no project capability. Builds on
fix-agy-link, hide-gone-folders and fix-feed-missed-changes, and merges after them.
