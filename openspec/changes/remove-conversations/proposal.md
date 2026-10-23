# Proposal

## Why

The owner wants to clean out unused or accidental conversations from Headquarter
(2026-10-23). Until now Headquarter only listed the agents' history and never changed
it.

## Expected Outcome

- In Add sessions, a row's ⋯ menu removes that conversation from its agent, the way the
  agent allows, with an Undo for the reversible ways.

## What Changes

- Add sessions: a ⋯ menu on each row, shown on hover; rows without one keep the same
  space so their ages line up.
  - Claude Code: Move to Trash. Claude Code has no command to delete one conversation
    (`claude purge` removes a whole project), so the transcript and the folder Claude
    keeps beside it (sub-agents, tool output) go to the system Trash.
  - Codex: Archive (`codex archive`, undone with `codex unarchive`) and Delete
    permanently (`codex delete --force`).
  - agy: no menu. agy deletes conversations only inside its own `/resume` picker (F4);
    in print mode it refuses interactive commands, and its conversations live in its own
    database, which Headquarter does not edit.
  - Never for a session running here or in another terminal.
- Move to Trash and Archive take effect after a 5-second toast with Undo; closing the
  window sends them at once. Delete permanently asks first and has no Undo.
- Host: `RemoveConversation { session, way }` checks the session is not running and the
  way is allowed, runs the removal, logs it (agent and way only), and stops offering the
  row.
- `AGENTS.md`: the one exception to leaving agent data alone.

Decisions (owner, 2026-10-23): control in Add sessions as a row ⋯ menu only (option C
of three in the local preview); an Undo toast, then the agents' own ways back (the
Trash, `codex unarchive`); no "Hide from Headquarter" (it removes nothing). Rejected:
driving agy's picker with keystrokes (fragile, leaves empty conversations), editing
agy's database, Finder's Trash (asks to control Finder; the file manager call does not).

## Non-goals

- Removing sessions in the sidebar (close them first), bulk removal, a command-line
  command.

## Capabilities

### Modified Capabilities
- `session-history`: one exception to leaving agent files unchanged.
- `session-sidebar`: removing past conversations from Add sessions.

## Impact

`crates/hq-protocol/src/lib.rs` (`RemoveConversation`, `RemoveWay`, `removal_ways`,
`ErrorCode::RemoveFailed`), `crates/hq-daemon/src/host/remove.rs` and `mod.rs`,
`crates/hq-signals/src/history.rs` (`claude_transcript`), `crates/hq-client`,
`crates/hq-app/src/modal/open_sessions.rs`, `crates/hq-cli` (`HQ_TRASH_DIR` for tests),
`AGENTS.md`. New dependency: `trash` 5.2 (MIT). Adds no project capability. Builds on
name-untitled-conversations and fix-agent-start-path and merges after them.
