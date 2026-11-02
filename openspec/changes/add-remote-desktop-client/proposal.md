# The desktop app as a client of a remote host

Status: approved by the owner 2026-11-02 ("approve all three").

## Why

The owner (2026-11-02) works on a Windows PC at home and a Mac elsewhere, and plans to move the
host to a headless server so any device they carry is only a client. Today the desktop app can
only reach a host on its own computer (`agentd.sock`), and the host's remote gateway was made
for phones: a narrow allow-list and one stream per connection. This change makes the desktop app
a full client of a host on another computer, the foundation for the Windows client
(`add-windows-client`) and the host-only install (`split-host-and-app-install`).

## Expected Outcome

- On a host, the owner shows a pairing code (the Remote Control window, or `hq remote pair` on a
  headless host). In the app on another computer, Connect to a Host takes the code and the app
  shows that host's sidebar, terminals, usage and notifications, as it does for a local host.
- A paired desktop may do everything the local app does, except stopping the host and pairing
  or removing devices (owner, 2026-11-02).
- A host switcher in the sidebar header names the host on screen; the owner switches between
  this computer and paired hosts, one at a time. The app reconnects by itself after a network
  drop.
- Quitting the app never stops a remote host; the folder browser lists the host's folders.

## What Changes

- Gateway: device kinds (phone, desktop), a desktop allow-list, several streams per connection
  (D1-D3).
- `hq-remote`: a blocking client link for desktop apps (D4); `hq-client` talks to a local socket
  or a remote link (D4).
- App: client key and known hosts (D5), Connect to a Host and the host switcher (D6), host-side
  answers for the folder browser (D7), local-only behaviour when the host is remote (D8).
- Protocol 15 (D9); `remote-control` and `desktop-app` specs.

## Non-goals

- Several hosts in one sidebar at once (later).
- Windows (`add-windows-client`) and the install split (`split-host-and-app-install`).
- Moving the phone core onto the new client link (later; the phone keeps its own link).
- Editing a remote host's agent settings from the app.

## Capabilities

No new project capability (`deployed` already covers the desktop app; the gateway stays the
only way in from other computers). Modified: `remote-control`, `desktop-app`.

## Impact

`hq-daemon` (gateway, a store migration for the device kind, a folder-listing request),
`hq-store`, `hq-protocol` (protocol 15), `hq-remote`, `hq-client`, `hq-app`. Protocol 15 means
the phone app and desktop app update together, as for 14.
