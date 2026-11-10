# Say why a paired host gave up, reconnect it, and keep its tag readable

Status: approved by the owner 2026-11-10 ("ok"), with Reconnect all as one menu row (owner, 2026-11-09).

## Why

Testing `v0.4.0-alpha.1` (2026-11-09), the owner's server showed only "not reachable" in the Hosts
menu while `hq remote status` on the server said remote control was on. The app had stopped trying
for good, most likely because the server still ran `0.2.0-alpha.2` (installed 2026-11-04,
protocol 15) while the app speaks protocol 16: once the owner updated the server's host and
reopened the app, the server connected. The app keeps the reason (`hosts.failed`) but shows it
nowhere: before
`add-merged-host-list` it was written under the sidebar, and now it is only written there when one
host is paired. The spec already asks for it ("Removed on the host": the Hosts menu says the host
no longer knows this computer), so this is a defect.

The same test found two more things. After a version mismatch the app never tries again, so
updating the older side is not enough: the app has to be reopened. And on a selected row with a
long title, the host tag shrank to "…" alone, which names nothing.

## Expected Outcome

- A paired host the app gave up on is named in the sidebar with what to do, for example
  "msigs63vr needs an update: it runs Headquarter 0.2.0-alpha.2, this app 0.4.0-alpha.1."
- The Hosts menu row says what is wrong in its state ("needs an update", "pair again",
  "not reachable") with the full reason under the row.
- After the older side is updated, the host's sessions come back within about 30 seconds without
  reopening the app, or at once with Reconnect all in the Hosts menu.
- A host tag on a narrow row keeps its first characters ("msig…").

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
- Reconnect all in the Hosts menu, while any paired host is not connected, tries them all at once
  (owner's choice, 2026-11-09).
- The row's host tag keeps at least four characters before its ellipsis.

## Non-goals

- The phone app (it shows one host and already says why it cannot connect).
- Updating a host from the app, or telling the host to update.
- Changing the protocol or what counts as a version mismatch.

## Capabilities

`desktop-app` only. No new project capability.

## Impact

`crates/hq-app` (`workspace.rs` watch loop and sidebar, `workspace/hosts.rs` Hosts menu,
`workspace/several.rs` state). No protocol, host or phone change. Ships in 0.4.0.
