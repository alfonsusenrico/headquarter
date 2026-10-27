# Design: status only from the session's own conversation

## Context

`hook_line` (`crates/hq-daemon/src/host/mod.rs`) picks the session from `payload.session`
(`HQ_SESSION`, set at launch), then falls back to the record's `session_id`. The status-line
path (`details_reported`) uses `payload.session` alone. Claude Code's daemon children inherit
both the environment and the `--settings` argument, so they report as their parent.

## Decisions

### D1. The conversation id is the check

For a Claude session the host knows the conversation up front: `--session-id <id>` for a new
one, `--resume <id>` for a resume (Claude Code 2.1.311 keeps the id on resume: the resumed
process and its transcript share it). `Hosted` keeps it in a `Mutex<Option<String>>`. A Claude
hook or status-line payload whose `session_id` differs is dropped.

Rejected: checking that the hook's process descends from the launched agent. The daemon is a
child of the agent that started it, so its sessions descend from it too.

### D2. Adopting a new conversation

- `SessionStart` with `source: "clear"` and a new `session_id`: the session switches to it and
  the index records it (`set_agent_session_id`). Claude Code sends this from the same process.
- A session launched with `--resume` and no id: the first payload's id is adopted.
- Any other differing id (a daemon session sends `source: "startup"`) is ignored.

### D3. Logging

`hook from another conversation id=<hq session id>`, ids only, at most once a minute per
session, so a chatty daemon session cannot fill the log.

## Risks

- A future Claude Code that changes the conversation id in place without `SessionStart`
  `clear` would lose hook detail until restart. Status still comes from OSC 7501 on the
  session's own terminal, so the sidebar keeps working.
