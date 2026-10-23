# Proposal

## Why

An agy session running in another terminal showed twice in Add sessions: once as an
unknown process ("agy pid …", in another terminal) and once as its conversation,
offered for adding (owner report, 2026-10-23). There was one agy process and one agy
conversation in that folder. agy does not report which conversation a process runs, so
Headquarter linked a process only to the single conversation in its folder that agy's
own index marked "not fully idle", read every 60 s when the index changed. While agy
waited for the owner's approval it wrote nothing, so a reading that caught the flag at
idle stayed in force and the link dropped.

## Expected Outcome

- An agy session in another terminal shows once, as its conversation, tagged "In
  another terminal", including while it waits for the owner.

## What Changes

- An agy process started with `--conversation <id>` is linked to that conversation.
- Otherwise it is linked to the conversation in its folder that agy wrote most recently
  since the process started, as Codex processes already are matched by folder and time.
- Two or more agy processes in one folder are not told apart by time and stay unlinked,
  unless named on their command line.
- The "not fully idle" flag is no longer read.

Decisions (owner, 2026-10-23, option A): rejected keeping the busy flag and reading it
more often (it can still read idle while agy waits).

## Non-goals

- agy's status for outside sessions stays unknown, as before.

## Capabilities

### Modified Capabilities
- `session-history`: how agy processes in other terminals are linked.

## Impact

`crates/hq-signals/src/external.rs` and `history.rs`, `crates/hq-daemon/src/host/outside.rs`,
`history.rs`, `mod.rs`. No protocol or index change. Adds no project capability.
Builds on fix-title-broadcast (same import code) and merges after it.
