# Design: status only from the session's own conversation

## Context

`hook_line` (`crates/hq-daemon/src/host/mod.rs`) picks the session from `payload.session`
(`HQ_SESSION`, set at launch), then falls back to the record's `session_id`. The status-line
path (`details_reported`) uses `payload.session` alone. Claude Code's daemon children inherit
both the environment and the `--settings` argument, so they report as their parent.

## Decisions

### D1. The conversation id is the check

`Hosted` keeps the Claude conversation it reports for: the launch's (`--session-id <id>`, or
`--resume <id>`), or none for `--resume` with Claude Code's picker. A Claude hook or status-line
payload from another conversation is dropped. The status-line helper now forwards the
conversation id beside the figures (`session_id`), which the host reads and does not keep.

Rejected: checking that the hook's process descends from the launched agent. The daemon is a
child of the agent that started it, so its sessions descend from it too.

### D2. The agent's own starts name the conversation

Found while building (2026-10-27): a resume can change the conversation id. Headquarter dev was
launched with `--resume 43096428...` and has since written to `582a1c38...`, while Pakem dev kept
its id. So the launch id is only provisional:

- The agent's first `SessionStart` (any source) names the conversation, and the session adopts
  it. The daemon's sessions start after the agent that spawned them, so they come later.
- `SessionStart` with `source` `clear` or `compact` adopts a new id at any time.
- A session that knows no conversation adopts the first one it hears of.
- Any other id is ignored. An adopted id is recorded in the index (`set_agent_session_id`).

### D3. Logging

`hook from another conversation id=<hq session id>`, ids only, at most once a minute per
session, so a chatty daemon session cannot fill the log.

## Risks

- A daemon session whose `SessionStart` reached the host before the agent's own would be taken
  for the session. Claude Code starts the daemon from the agent, after the agent's start.
- A future Claude Code that changes the conversation id in place without a `SessionStart` would
  lose hook detail until restart. Status still comes from OSC 7501 on the
  session's own terminal, so the sidebar keeps working.
