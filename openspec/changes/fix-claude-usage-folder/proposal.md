# Claude usage reads in a folder Claude trusts

Status: plan, waiting for the owner's approval.

## Why

The owner (2026-10-29, 0.1.20): Claude usage stuck at "Couldn't update since 3:16 PM" with the
trust message; Refresh spun, changed nothing, then seemed dead. The hidden `/usage` read runs in the
most recent Claude session's folder, which was the home folder; Claude Code never keeps trust there,
so every read stopped at its question. A click within 30 s of a failed read did nothing
(`research/evidence/claude-usage-folder/findings.md`).

## Expected Outcome

- Claude usage keeps updating while the owner works in Claude Code in the home folder (or any
  folder Claude does not trust).
- After a failed read, Refresh always reads again.

## What Changes

- Host: the hidden read runs in the most recent session folder that Claude Code itself records as
  trusted (D1).
- Host: the refresh floor applies after a good reading only (D2).
- The trust message names what helps (D3).
- `provider-quota` spec.

## Non-goals

- Answering Claude's trust question (still never answered).
- Making Refresh faster: it still waits for every agent's read (agy's takes about 8 s).

## Capabilities

No new project capability. Modified: `provider-quota`.

## Impact

`hq-daemon` (`host/usage.rs`, a reader for one field of `~/.claude.json`). No protocol change.
Checked against Claude Code 2.1.312.
