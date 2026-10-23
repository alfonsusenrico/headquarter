# Proposal

## Why

On 2026-10-22 the owner made sessions running in another terminal unpickable in Add
sessions. Using 0.1.1 showed the cost (owner, 2026-10-23): to keep such a conversation
in Headquarter, the owner must first close it in the other terminal, then come back
and add it. The owner wants to add it while it still runs there, and continue it in
Headquarter later.

The host and `hq add` already accept a session running elsewhere when its
conversation is known, and the sidebar already shows such a session as "ext" and
observed only. Only the app's Add sessions window refuses it.

## Expected Outcome

- In Add sessions, a session running in another terminal with a known conversation
  can be picked. It stays in date order, tagged "In another terminal", no longer
  greyed. Added, it shows in the sidebar as "ext" with its live status.
- When the owner closes it in the other terminal while it is shown, the pane says
  "Closed in the other terminal." with an "Open here" button. Nothing starts until
  the owner chooses it.
- An outside agent with no known conversation stays greyed and unpickable: there is
  nothing to continue later.

## What Changes

- `hq-app` (`modal/open_sessions.rs`): a linked outside session is pickable, tagged
  "In another terminal", not greyed. The info popup about outside sessions, which
  nothing has opened since its section was removed on 2026-10-22, is removed.
- `hq-app` (`workspace.rs`): the details pane tells apart a session that was closed
  in another terminal while shown, with "Open here".

Decisions (owner, 2026-10-23): add it now and open it here later (rejected: taking
it over by closing the other terminal, which would signal an outside process; and
offering both); date order with the tag (rejected: the old section on top); wait for
the owner when the other terminal closes it (rejected: continuing it at once).

## Non-goals

- Headquarter never attaches to, signals, or ends the outside process; the boundary
  in `AGENTS.md` is unchanged.
- No change to the host, the protocol, `hq add`, or the index.

## Capabilities

### Modified Capabilities
- `session-sidebar`: Add sessions offers sessions running in another terminal when
  their conversation is known; the pane says when such a session was closed there.

No project capability is added.

## Impact

`crates/hq-app/src/modal/open_sessions.rs`, `crates/hq-app/src/workspace.rs`, and
their tests. Verified against the agent versions installed on 2026-10-23 (Claude Code
2.1.309 seen in the app).

## Approval

Approved by the owner on 2026-10-23 ("yes go ahead").
