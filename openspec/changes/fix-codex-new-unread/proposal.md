# Codex `/new` no longer marks a session unread

Status: approved by the owner 2026-11-09 ("do it all"; Phase 1 of
`research/notes/2026-11-09-roadmap-plan.md`).

## Why

Codex's `/new` reports `working`, then `idle` (Codex 0.166.0,
`research/evidence/codex-new/2026-11-09.md`). Headquarter counts Codex's `idle` as a finished turn,
a rule from Codex 0.163.0, which ended turns with `idle` (`hq-signals/src/adapters/mod.rs`). So a
session the owner already used shows "idle, not read yet" after `/new`, with nothing new to read.
Codex has ended real turns with `done` since 0.164.0.

## Expected Outcome

`/new` in a Codex session leaves it idle. A Codex turn that finishes still marks the session
unread, and still makes the host read Codex usage again.

## What Changes

- Codex's `idle` report counts as a stop, as for Claude Code and agy; only `done` ends a turn (D1).
- `agent-status` "Unread completion": a scenario for a stop with nothing to read.

## Non-goals

- Codex 0.163.0 and older, which ended turns with `idle`: they would no longer mark turns unread.

## Capabilities

No new project capability. Modified: `agent-status`. Codex 0.166.0 (`/new`); a real 0.166.0 turn
to confirm with the owner's go.

## Impact

`hq-signals` (`adapters/mod.rs`); the host's Codex stand-in in tests ends turns with `done`.
