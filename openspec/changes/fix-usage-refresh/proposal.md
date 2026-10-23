# Proposal

## Why

Pressing Refresh in the usage details closed them (owner, 2026-10-23). The details
sit over a backdrop that closes them on any press outside, and they did not block the
mouse, so the press on Refresh also reached the backdrop. Even when they stayed open
there was nothing to watch: the host answered a refresh at once and read in the
background, so the app could not tell when the new numbers were in.

## Expected Outcome

- Refresh keeps the details open, shows "Refreshing…" with a spinner, and the new
  numbers when the readings are in.

## What Changes

- The details block presses from reaching the backdrop behind them.
- The host answers `RefreshUsage` once no usage read is under way (at most 40 s, the
  hidden `/usage` read's own limits with room to spare). The protocol is unchanged;
  the answer only comes later.
- While its refresh is unanswered, the app shows the Refresh button as "Refreshing…",
  disabled, with a spinner (no spinner when the system asks for reduced motion). A
  failed refresh shows the host's error as other failed requests do.

Decisions:
- **The host answers when the readings are in.** It is the only side that knows.
  Rejected: the app waiting for the next usage change (none comes when a refresh is
  skipped because a reading was just taken, so the spinner would never stop).
  Rejected: a fixed-length spinner (says nothing true about the reading).
- **The UI library's button spinner.** Rejected: the agents' marks lighting up, used
  while sessions load: too large for a button.

## Non-goals

- What is read, and how often, is unchanged.

## Capabilities affected

- provider-quota: refresh keeps the details open and shows its progress.

## Impact

- `crates/hq-app/src/workspace.rs`, `crates/hq-daemon/src/host/usage.rs`,
  `crates/hq-daemon/src/host/mod.rs`. Adds no project capability.
