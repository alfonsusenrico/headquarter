# Every host's sessions in one list

Status: approved by the owner 2026-11-09 ("approve, build it"; phase 2, step 2; the outline in
`research/notes/2026-11-09-merged-host-list-plan.md` was approved 2026-11-09). Builds on
`add-shared-host-endpoint`.

## Why

The owner works between this Mac and a server, keeping a long-lived agent on the server
(2026-11-04). The desktop app shows one host at a time: the server's sessions and the Mac's never
share the list, notifications come only from the host on screen, and groups cannot hold sessions
from both. Decisions: `research/notes/2026-11-04-several-hosts.md` (2026-11-04, -05, -09).

## Expected Outcome

- One sidebar list with the sessions of this computer and of every paired host. A session on
  another computer shows that computer's name; on a computer without its own host (Windows) every
  session shows its host's name.
- Groups merge by name across hosts; moving a server session into "ops" puts it with the Mac's
  "ops" sessions.
- The sidebar header becomes a Hosts menu: each host with its state, Connect to a Host…, and
  Forget, which asks in place before forgetting.
- A host that cannot be reached keeps its sessions in the list, greyed, with "Reconnecting…".
- New Session asks which host when there is more than one.
- The usage footer and the Usage window show the selected session's host, named when not this
  computer.
- Notifications come from every host, named when not from this computer.

## What Changes

- The workspace follows one feed per host and keeps one merged list, with the host of each
  session (D1, D2).
- Groups: a merged view by name over each host's groups; group actions go to every host holding
  the name (D3).
- Actions, terminals, the folder browser and Add sessions go to the session's host (D4).
- Usage per host; the footer follows the selection (D5).
- The Hosts menu with Forget confirmed in place (D6).
- Specs: `desktop-app` ("Host switcher" removed; one list, the Hosts menu, groups across hosts,
  usage, new sessions on a host added; reconnecting, client, quitting modified); `notifications`
  (the host's name).

## Non-goals

- The phone's list (one computer at a time until a later change).
- Moving a session from one host to another.
- Usage merged across hosts that share an account.

## Capabilities

No new project capability. Modified: `desktop-app`, `notifications`.

## Impact

`hq-app`: `workspace.rs` (feeds, list, routing), `workspace/groups.rs`, `workspace/hosts.rs`,
`sidebar/model.rs` (labels), `notifications.rs`, the usage footer and window, New Session.
No protocol or host change: every request goes over its host's link (step 1).
