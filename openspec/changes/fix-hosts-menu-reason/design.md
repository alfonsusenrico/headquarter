# Design: say why a paired host gave up

## Context

Each paired host has its own feed thread (`watch_sessions` in `workspace.rs`). A failure ends in
`Followed::Final` when trying again cannot help (`LinkError::NotKnown`, `LinkError::Mismatch`) or
when the host cannot even be built (`hosts::host` fails). The thread then sends
`ListEvent::Failed(message)` and stops. The workspace puts the message in `hosts.failed` and
removes the host's sessions. The Hosts menu prints only "not reachable" (`state_word`), and the
sidebar prints the message only for this computer or when one host is paired.

## Decisions

### D1. A failure keeps its kind

`ListEvent::Failed` carries a `HostTrouble { kind, message }`. The kinds are `Update`
(versions differ), `PairAgain` (the host removed this computer), and `Other` (anything else).
The state word and the retry depend on the kind, not on parsing the message.

Rejected: keeping a plain string and matching its text. Fragile, and the wording is the
user's, not a contract.

### D2. The sidebar names it

One amber line per troubled host goes under the "Reconnecting to …" line, sorted by host name:

- Update: "<host> needs an update: it runs Headquarter <host version>, this app <app version>."
  When the app is the older side (the host's protocol is higher): "Update this app: <host> runs
  Headquarter <host version>, this app <app version>."
- PairAgain: "<host> no longer knows this computer. Pair it again with Connect to a Host."
- Other: "Can't use <host>: <message>"

The line stays until the host lists its sessions again or is forgotten. This computer's own host
failure keeps its current place (the sidebar message), unchanged.

Rejected: the Hosts menu only. The owner looked at the list, saw the server's sessions missing,
and had no hint until opening the menu, which also said too little.

### D3. The Hosts menu says the kind and the reason

The state word becomes "needs an update", "pair again" or "not reachable" by kind. Under the row,
the reason from D2 in 12 px muted text, wrapping within the menu's 300 px.

### D4. Versions that differ are tried again

On `Update`, the feed thread sends the trouble, then keeps trying every `RECONNECT_MOST`
(30 seconds) while the window listens, without the "Reconnecting" state. When the host answers on
the same protocol, the list arrives and `hosts.failed` is cleared for it (as `ListEvent::List`
already does). `PairAgain` stops as now: only pairing again helps, and Connect to a Host restarts
the feed. `Other` from `hosts::host` stops as now (the paired host's record or this computer's key
cannot be read).

Rejected: retrying every kind. A removed computer would knock on the host every 30 seconds for
nothing.

### D5. Reconnect all

The owner chose one row in the Hosts menu (2026-11-09), above Connect to a Host, shown while any
paired host is not connected (connecting, reconnecting, needs an update, pair again, not
reachable). It ends each such host's feed and starts a new one at once, clearing its trouble
first, so the menu shows "connecting" until the host answers. A host that removed this computer
comes back as "pair again" when the new try is refused the same way.

Rejected: a "Try again" on each row (more buttons for a rare action), and an icon in the sidebar
header (always visible for something needed rarely).

### D6. The tag keeps the name's first characters

Found in the alpha test (2026-11-09): on a selected row the close button takes width, the tag
shrank to its 28 px minimum and showed only "…", sitting low. The tag's minimum becomes the width
of its first four characters plus the ellipsis and padding, measured with the row's font, and the
tag stays centred. The title gives way first. Names of four characters or fewer show whole.

Rejected: hiding the tag on a narrow row. Which computer a session runs on matters most when the
row is selected.

## Failure behaviour

- The host updates while the app runs: within 30 seconds its sessions return, the line goes.
- The host is offline while versions also differ: the dial fails before any version check, so it
  shows as "Reconnecting" until it answers, then as needing an update.
- The owner forgets the host: its line and state go with it (`drop_host` already clears
  `hosts.failed`).

## Risks

- A host on an older release, kept on purpose, is dialled every 30 seconds. Each dial is one
  connection: the app reads the host's welcome, sees the protocol differ and closes ("versions
  differ"), and the host's log may show a connect and a disconnect each time. Acceptable for a
  personal setup.
