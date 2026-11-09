# Say why a paired host gave up, and come back after an update

Status: proposed 2026-11-09, waiting for the owner's approval.

## Why

Testing `v0.4.0-alpha.1` (2026-11-09), the owner's server showed only "not reachable" in the Hosts
menu while `hq remote status` on the server said remote control was on. The app had stopped trying
for good. The likely reason is that the server still runs `0.2.0-alpha.2` (protocol 15) while the
app speaks protocol 16. The app keeps the reason (`hosts.failed`) but shows it nowhere: before
`add-merged-host-list` it was written under the sidebar, and now it is only written there when one
host is paired. The spec already asks for it ("Removed on the host": the Hosts menu says the host
no longer knows this computer), so this is a defect.

A second gap: after a version mismatch the app never tries again, so updating the older side is not
enough. The app has to be reopened.

## Expected Outcome

- A paired host the app gave up on is named in the sidebar with what to do, for example
  "msigs63vr needs an update: it runs Headquarter 0.2.0-alpha.2, this app 0.4.0-alpha.1."
- The Hosts menu row says what is wrong in its state ("needs an update", "pair again",
  "not reachable") with the full reason under the row.
- After the older side is updated, the host's sessions come back within about 30 seconds without
  reopening the app.

## What Changes

- The app keeps why each host gave up as a kind (versions differ, removed there, other) with its
  message.
- The sidebar shows one amber line per such host, under the "Reconnecting to …" line, with the
  short reason and what to do.
- The Hosts menu row shows a state word per kind and the reason in small muted text under it.
- When versions differ, the app tries again every 30 seconds, quietly. When the host answers on the
  same protocol, its sessions return and the line goes away.
- A host that removed this computer is not retried. Pairing it again with Connect to a Host starts
  it again, as now.

## Non-goals

- The phone app (it shows one host and already says why it cannot connect).
- Updating a host from the app, or telling the host to update.
- Changing the protocol or what counts as a version mismatch.

## Capabilities

`desktop-app` only. No new project capability.

## Impact

`crates/hq-app` (`workspace.rs` watch loop and sidebar, `workspace/hosts.rs` Hosts menu,
`workspace/several.rs` state). No protocol, host or phone change. Ships in 0.4.0.
