# Proposal

## Why

The owner saw a Claude Code session show idle while one of its background shell commands
ran (2026-10-24), against "Background work counts as working". A dev host on `main`
reproduced it: idle from the end of the turn until the background command finished.
Logging in a dev build (not kept) showed the scan dropped the session: Claude Code
2.1.310 writes the status `shell` in its session record while background commands run
after a turn, a word Headquarter did not know (it knew `busy`, `waiting`, `idle`, seen up
to 2.1.306).

## Expected Outcome

- A hosted Claude Code session shows working while its background shell commands run,
  then idle not read yet after Claude's follow-up turn, as the spec says.

## What Changes

- `hq-signals` (`external.rs`): `shell` in a Claude Code record or `claude agents --json`
  reads as working.

## Non-goals

- Other unknown status words still read as unknown (no guess), as before.

## Capabilities

### Modified Capabilities
- `agent-status`: Claude Code's `shell` status counts as background work.

No project capability is added.

## Impact

`crates/hq-signals/src/external.rs`, `crates/hq-cli/tests/host.rs`. Observed with
Claude Code 2.1.310 (`research/evidence/claude-shell-status/`).

## Approval

Approved by the owner on 2026-10-24 ("Approve").
