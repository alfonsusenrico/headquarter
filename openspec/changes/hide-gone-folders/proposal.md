# Proposal

## Why

Add sessions listed 98 conversations whose folder no longer exists (owner report,
2026-10-23): 69 agy runs in temporary `af-eval-*` folders made by automated
evaluations, 16 from development folders under `/tmp`, and 13 others. None can be
continued, since continuing needs the folder, and they crowd out the owner's own work.
None of them was in the sidebar.

## Expected Outcome

- Add sessions lists only conversations whose folder still exists, and its counts
  agree.

## What Changes

- The Add sessions window leaves out conversations whose folder is gone, when it opens
  and on every update. A session with no known folder stays.
- Nothing is deleted: the agents' own history and Headquarter's index are untouched, and
  `hq ls --all` still lists every conversation.

Decisions (owner, 2026-10-23, option B): rejected deleting the conversations from the
agents' own stores (it would change another program's data and agy has no command for
it), and rejected filtering by folder name (temporary folders have no fixed pattern).

## Non-goals

- Sessions in the sidebar whose folder was removed stay in the sidebar.

## Capabilities

### Modified Capabilities
- `session-sidebar`: Add sessions offers only conversations that can be continued.

## Impact

`crates/hq-app/src/modal/open_sessions.rs`. Adds no project capability. Builds on
mark-external-sessions-unaddable (same window code) and merges after it.
