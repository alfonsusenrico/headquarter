# Design: one connection point for every paired host

## Context

`Link` (`hq-remote/src/client.rs`) owns a tokio runtime and binds its own endpoint with the app's
`client.key`, with a relay map made from the host's relay. `Inner::shut` closes the connection and
the endpoint, "so this computer's key is free again". The app keeps at most one link open.

## Decisions

### D1. One client endpoint per app process

`ClientEndpoint`: the app's key, one tokio runtime, and one endpoint bound lazily with n0's
default relays (`Relays::Default`; `Disabled` under `HQ_REMOTE_RELAYS=none`). Every `Link` dials
through it. A host on another relay (a personal one) is reached by its address' relay URL: iroh
opens a connection to any relay URL, the map only gives auth tokens (spike, point 4).

Rejected: one endpoint per host with distinct keys. Each host pairs a key; a key per host would
need pairing again and more state for nothing the shared endpoint lacks.

### D2. A link owns only its connection

`Link` keeps its address, its connection and its control stream; `close` and `Drop` close the
connection, never the endpoint. The rebind-on-relay-change logic goes: the endpoint's relays no
longer depend on the host. Redials after a drop use the same endpoint.

### D3. The app keeps every paired host's link

`hosts.rs` makes a link per known host on first use and keeps it; `show()` and `pair()` stop
closing the others; `forget()` closes that host's link. A connection idle while its host is not on
screen stays open through QUIC keep-alive; if it drops, the next use redials, as today.

## Failure behaviour

- The endpoint cannot bind: every remote host reports unreachable, as a link does today.
- A host on a relay outside n0's map is idle beyond iroh's 60 s cleanup of unused relay
  connections: the next use reopens it; to check (spike's open point).

## Risks

- Keeping every paired host connected costs a QUIC connection and its keep-alive per host; with
  a handful of hosts this is small.
