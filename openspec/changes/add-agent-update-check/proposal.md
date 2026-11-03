# Headquarter checks an agent after it updates

Status: approved by the owner on 2026-11-02 ("approve both"); building.

## Why

The owner (2026-11-02): agent CLIs update often, and an update can silently break Headquarter's
reading of them (Claude Code 2.1.316 dropped its five-hour window, and every Claude usage read
failed for two days before anyone noticed). Headquarter should notice an agent's new version,
check it by itself without spending tokens, and say plainly when something no longer works.

## Expected Outcome

- When an installed agent's version changes, the host runs free checks within a few minutes (no
  prompt, no model call): the agent reports its status at start (OSC 7501), and its usage reads.
- If a check fails, a quiet notice names the agent, the new version and what no longer works
  ("Claude Code 2.1.316: Headquarter can't read its usage"), and stays until dismissed; the host log
  records every check. Passing checks stay silent.
- A repository script runs the same checks plus one real turn per agent, on the owner's go, and
  writes a dated report under `research/evidence/agent-checks/`.

## What Changes

- Host: per-agent version memory and the free checks on change (D1, D2).
- App: a notice for a failed check (D3), reusing the quiet notice.
- `hq agents --check` and `scripts/agent-check` (D4).
- `agent-discovery` spec.

## Non-goals

- Fixing a broken agent automatically (each break is its own change).
- Real turns without the owner's go.

## Capabilities

No new project capability. Modified: `agent-discovery`.

## Impact

`hq-daemon` (a checker beside the usage readers), `hq-protocol` (an optional field for check
results and a local-only request), `hq-cli`, `hq-app` (the notice), `scripts/`. Protocol stays 15:
older apps ignore the field, and the request is refused to phones and other computers.
