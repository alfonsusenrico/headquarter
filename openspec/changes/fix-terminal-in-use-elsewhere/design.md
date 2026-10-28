# Design

## Context

`add-remote-gateway` D6: a live session's terminal takes the size of the viewer in use, which is
the one that most recently attached, typed, or sent `Focus {}`. Other viewers keep their size for
when they are in use. Viewers are not told any of this today. A phone's connection reaches the
host's socket through the remote proxy (`proxy` in `host/remote.rs`), so the host sees it as one
more local client and does not know it is a phone.

Owner decisions (2026-10-28): say so on the device not in use, with one tap or click to take
over. Plan first, build after approval. No transcript view.

## Decisions

### D1. The host tells each viewer who is in use

A new response `InUse { holder }`, with `holder` one of `Here`, `Computer` (a viewer on the
computer itself), or `Phone { name }`. The host sends it to a viewer when it attaches, and to every
viewer of the session whenever the viewer in use changes (attach, typing, `Focus {}`, the viewer in
use leaving). It applies to agent sessions and their shells, which share the same code.

Rejected: each client guessing from its own actions (it cannot see the other device). Rejected: a
field on `SessionInfo` (the session list goes to every client, and "in use here" differs per
viewer).

### D2. The proxy names the phone

The remote proxy sends a new request `Device { name }` on the phone's behalf right after the
phone's `Hello`, with the name from the paired device's index row. The host keeps it for that
connection's viewers. `allowed()` refuses `Device` from the phone itself, so a phone cannot claim
another name or claim to be the computer.

Rejected: a field on `Hello` (`Hello` keeps one shape in every protocol version so any client can
stop any host). Rejected: the phone sending its own name (the host must not trust it).

### D3. The phone covers the terminal

While the holder is not `Here`, the phone covers the terminal area with "In use on <computer>"
(the paired computer's name; "In use on <phone>" when another phone holds it) and a "Use here"
button that sends `Focus {}`. The header, the key row and the text field stay usable. Typing or a
key there takes the session over as today. The terminal under the cover keeps receiving output,
and the agent redraws at the phone's size once it is in use.

### D4. The desktop shows a notice

While the holder is a phone, the terminal pane (and the shell panel when its shell is held) shows a
slim notice at its top right, clear of the narrower drawing, which starts at the left, "In use on <phone> · Click to use here". It is
drawn over the terminal rather than beside it, so the pane's grid, and the size it keeps for when
it is in use, do not change. The narrow drawing stays visible below it. Clicking the notice sends
`Focus {}`, as a click in the terminal does today. When the holder is `Computer` (another local
client such as `hq attach`), it says "In use in another terminal · Click to use here". It uses the
terminal's own text and background colours swapped, so it is as readable as the terminal's text in
every theme.

### D5. Protocol 14

The new response and request change the protocol to 14. A phone and a computer on different
versions already get "Update Headquarter on <computer>" or the app update notice.

## Risks

- A device that misses an `InUse` message would show a stale state. Mitigation: the host sends it
  on every attach and every change, and a reconnect attaches again.
- Brief flicker of the cover when typing on the phone hands the session over. Acceptable: it lasts
  one message.

## Open questions

- None.
