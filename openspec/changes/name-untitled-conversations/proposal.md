# Proposal

## Why

Past conversations whose agent never wrote a title showed their generated name, such as
`claude-e5a1fa64`, which reads like a leftover (owner report, 2026-10-23). They are real
conversations (on the owner's machine, 5 Claude Code conversations with a prompt and a
reply but no title). Headquarter shows only titles the agents write and never reads
prompts, so it cannot make one up.

## Expected Outcome

- Such a conversation reads "Untitled conversation", followed by its short id in muted
  text in Add sessions ("Untitled conversation · e5a1fa64"), so it stays identifiable
  and `hq resume` still takes `claude-e5a1fa64`.

## What Changes

- `hq-protocol` (`sidebar.rs`): `untitled_id` gives the short id of a past conversation
  with no title and no name from the owner; `display_title` reads
  "Untitled conversation · <id>" for it. Sessions started in Headquarter and named ones
  are unchanged.
- `hq-app` (`modal/open_sessions.rs`): the row shows "Untitled conversation" and the id
  muted after it. The sidebar, header and search use the combined text.

Decision (owner, 2026-10-23): "Untitled conversation" with the short id. Rejected:
keeping the generated name; a title made from the first prompt (Headquarter never reads
prompts).

## Non-goals

- Generated names in the index and in `hq ls` stay as they are.

## Capabilities

### Modified Capabilities
- `session-history`: how a past conversation without a title is shown.

## Impact

`crates/hq-protocol/src/sidebar.rs`, `crates/hq-app/src/modal/open_sessions.rs`. Adds no
project capability. Builds on offer-continuable-sessions (same window code) and merges
after it.
