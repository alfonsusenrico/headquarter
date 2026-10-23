# Proposal

## Why

The usage strip showed agy as unavailable: in agy 1.3.6 its usage reached only its own
status line, configured in its settings files, which Headquarter does not change
(add-status-probe). The owner asked to check again (2026-10-23). agy 1.4.4 documents a
headless `agy -p "/usage"` whose JSON output is meant for scripts. One run in an empty
folder took about 8 s, started no conversation, used no model tokens, and added nothing
to agy's input history (`research/evidence/agy-usage/2026-10-23-headless-usage.md`).

## Expected Outcome

- agy's ring fills like Claude's and Codex's, and its details list every pool of
  models it reports, such as "5h · Gemini" and "week · Claude and GPT".
- agy turns red at 80% and is named or counted in the label like the others.
- A failed read says so in the same voice as Codex and Claude; an agy Headquarter
  cannot find reads "Agy usage not available".

## What Changes

- `hq-signals` (`quota.rs`): `read_agy` runs `agy -p /usage --output-format json` with a
  30 s limit and parses `command.data.groups[].buckets[]` into windows (5h, week),
  each carrying its pool's name ("Gemini Models" → "Gemini").
- `hq-daemon` (`host/usage.rs`): agy is read on refresh, every 10 minutes while a client
  shows usage, and 10 s after a hosted agy turn ends; never while nothing shows usage.
  It runs in an empty folder of Headquarter's (`quota/agy-workspace`). The last good
  reading is kept in `quota/agy.json`, so a restart shows it at once. A refresh waits
  for the agy read too.
- `hq-protocol` (`usage.rs`): `ring_windows` gives the windows the rings show: the
  all-models ones, or, for an agent that reports only pools, the pool nearest its
  limit. The ring tooltip uses the same windows.
- `hq-probe`: `quota` reads agy the same way.
- Tests stub `agy` wherever usage is read, so they never start the machine's own.

Decisions (owner, 2026-10-23):
- **Read agy's usage with its own headless command.** Rejected: its status line (would
  mean changing agy's settings files).
- **The rings follow the pool nearest its limit**, by the label's rule (highest whole
  percentage, the shorter window on a tie); on equal pools the first agy reports.
  Rejected: always the Gemini pool (a nearly spent Claude/GPT pool would stay hidden).
- **Every pool in the details**, as Claude's per-model windows are.
- **Same cadence as Claude's hidden read**: 10 minutes, on refresh, after a hosted turn.

## Non-goals

- agy sessions in other terminals do not trigger a read; agy reports no turn ends there.
- What the account behind agy's limits is: agy does not say.

## Capabilities

### Modified Capabilities
- `provider-quota`: agy's usage is read and shown.

## Impact

`crates/hq-signals/src/quota.rs`, `crates/hq-daemon/src/host/usage.rs`, `host/mod.rs`,
`crates/hq-protocol/src/usage.rs`, `crates/hq-app/src/workspace.rs`,
`crates/hq-probe/src/report.rs`, `crates/hq-cli/tests/host.rs`. New files in the host's
data folder: `quota/agy.json`, `quota/agy-workspace/`. No protocol or index change.
Adds no project capability.

This change builds on fix-agent-start-path, show-both-usage-windows and
fix-usage-refresh, and merges after them.

## Approval

Approved by the owner on 2026-10-23 ("ok implement it").
