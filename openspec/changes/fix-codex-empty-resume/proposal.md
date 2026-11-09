# An empty Codex session starts fresh

Status: approved by the owner 2026-11-09 (chose "Start fresh" for a Codex session where nothing
was sent).

## Why

Seen 2026-11-09 on a test host: after the host restarted, a Codex session in which nothing had
been sent opened Codex's resume picker, listing the owner's other Codex conversations
(`research/evidence/image-upload/2026-11-09-phone-check.md`). Codex reports a conversation id only
once one exists, so such a session has none, and resuming without an id runs `codex resume`, the
picker (`hq-signals/src/adapters/codex.rs`). Picking one there would tie another conversation to
this session.

Conversations imported from Codex's history always carry their id; a Codex row without one is a
session Headquarter started where nothing was sent.

## Expected Outcome

Resuming such a session, from the app, the phone or `hq resume`, or the host starting it again,
opens a new Codex conversation in its folder. Sessions with a known conversation still resume it.

## What Changes

- Codex's launch: resuming without a conversation id starts Codex without `resume` (D1).
- `session-host` "Resume": the picker stays for Claude Code only.

## Non-goals

- Keeping a session's launch options across a resume: they can carry a prompt, which the index
  never stores; they stay out on purpose.
- Claude Code: its sessions always carry an id (chosen at start), and one never used already
  starts afresh.

## Capabilities

No new project capability. Modified: `session-host`. Codex 0.166.0.

## Impact

`hq-signals` (`adapters/codex.rs`). No stored data or protocol change.
