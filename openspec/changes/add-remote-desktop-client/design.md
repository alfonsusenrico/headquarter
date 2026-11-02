# Design: the desktop app as a client of a remote host

## Context

`host/remote.rs` runs an iroh endpoint while a device is paired or a code is shown. It accepts
one bidirectional stream per connection, pairs with a one-time code, then proxies frames to a
new `agentd.sock` connection through `allowed()` (phones). The app opens many host connections:
the session feed, one per terminal, and short ones per action (`hq-client`, `terminal/session.rs`,
`workspace/*`), all over `UnixStream`. The folder browser (`modal/folders.rs`) and first-run
setup read the local machine.

## D1. Device kinds

`Request::Pair` gains an optional `kind` (`phone` or `desktop`; absent means phone, so phone apps
of protocol 14 pair as before). Store migration `0010_device_kind.sql` adds `kind` to the devices
table (default `phone`). The Remote Control window and `hq remote devices` show the kind.
Rejected: a separate pairing path for desktops (two code paths to keep safe).

## D2. What a desktop may do

`allowed(kind, request)` stays one exhaustive match. A desktop may send everything the local app
sends except `Stop`, `RemotePair`, `RemoteSet`, `RemoteDevices`, `RemoteRemove` and the
gateway's own `Pair`, `Device` and `Unpair` (owner, 2026-11-02: a lost laptop cannot add
devices or stop the host). Phones keep today's list. A phone can already open a shell, so the
wider desktop list adds actions on sessions and groups, not new reach into the computer.

## D3. Several streams on one connection

After pairing or the welcome on the first stream, the gateway keeps accepting bidirectional
streams on the same connection; each is proxied to its own `agentd.sock` connection with the
same device name and checks. At most 64 open streams per connection; the 65th is refused and
logged. The connection map stays keyed by device, so removing a device closes all its streams.
Rejected: one QUIC connection per app connection (a handshake and relay path per terminal).

## D4. The client link

`hq-remote` gains `client`: a blocking `Link` that owns a small tokio runtime thread, the
client's endpoint and one connection to a host. `Link::open()` returns a blocking
reader and writer bridged to a new stream. It reconnects with backoff (1 s doubling to 30 s)
and reports its state. `hq-client` gets a `Transport` enum (local socket, remote link); `Client`
holds boxed reader and writer, so callers keep their blocking code. The phone keeps
`hq-mobile/src/link.rs` for now (recorded as debt: two links; consolidate later).

## D5. Client key and known hosts

The app keeps `client.key` (iroh secret, mode 600) and `hosts.json` (name, endpoint id, relay,
addresses, paired-at; no secrets beyond the key) in its own data folder
(`~/.local/share/headquarter-app/`; `%APPDATA%\Headquarter` in `add-windows-client`), apart from
the host's data. `app-state.json` remembers the host last shown.

## D6. Connect to a Host and the switcher

- The sidebar header shows the host on screen ("This Mac" or the host's name) as a button; it
  opens the host list: this computer (when it has a host), each paired host with its state,
  Connect to a Host…, and Forget for a paired host.
- Connect to a Host… is a window with one field for the code (pasted; the phone's QR is not
  needed on a desktop) and the result: paired, wrong or expired code, unreachable.
- Switching closes the terminals and reconnects the workspace to the chosen host. While a
  remote host is unreachable the sidebar says "Reconnecting to <host>…" and keeps the last list
  greyed; terminals reattach when it returns.
- Notifications name the host when it is not this computer.

## D7. Answers from the host

- New request `ListFolder { path }` → `Folder { home, path, folders }`: directory names only,
  hidden ones left out as the browser does today (an empty path is the home folder, so the
  answer also names it). `FindFolders {}` walks the host's home folder for finding a folder by
  name and answers in `FoundFolders` batches on the same connection, the last one `done`. The
  listing and the walk move from the app to `hq_discovery::folders`, so the app reads its own
  computer directly and the host answers remote clients with the same code. Phones may send
  neither.
- First-run setup runs only for this computer's host; a remote host's agents are its own. The
  app's look settings stay local to the app's computer.
- The Usage window's total names where each part runs ("Headquarter on <host>").

## D8. What stays local

- Quit with a remote host on screen quits the app only, without the stop question.
- The updater updates only this computer and never stops a remote host.
- The Remote Control window manages this computer's host only; it is hidden while a remote
  host is on screen.
- Dropping files into a remote terminal shows a notice ("files are on this computer, the
  session is on <host>") instead of pasting a local path. File links open only when the host
  is this computer.

## D9. Protocol and versions

Protocol 15 (the device kind, `ListFolder`). A desktop whose protocol differs from the host's
gets the existing mismatch refusal; the app shows both versions and which side to update.

## Logging

The gateway logs `remote connected device=<name> kind=<kind> path=…`, stream counts on close,
and refusals by request kind; never codes, keys, input or output.

## Risks

- A wider allow-list: covered by the exhaustive match and host tests for every refused kind.
- Relayed terminals feel slower: measure keystroke round trip over a relay and direct.
- Blocking bridges deadlocking on close: tests for host restart, network drop and app quit
  mid-stream.
