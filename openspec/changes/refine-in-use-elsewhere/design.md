# Design

## Context

`fix-terminal-in-use-elsewhere` (this branch) tells each client who has the session
(`InUse { holder }`), names the phone through the remote gateway, draws a notice over the desktop's
terminal and a cover on the phone. The owner chose instead (2026-10-28): option D1 on the desktop,
no cover on the phone, and the phone closing to home when the desktop takes the session back.

## Decisions

### D1. The desktop shows an inactive-style page, not the terminal

While the holder is a phone (or another client on the computer), the pane shows the Details layout
used for inactive sessions: the agent's mark, "Open on <phone>" (or "Open in another terminal"),
"The agent is drawing for the phone's screen." and a primary "Use here" button. Enter in the pane
does the same. Other keys do nothing and never reach the agent. The desktop stays attached
underneath, so its terminal is current the moment it takes the session back. The shell panel does
the same for its shell.

Rejected: D2 (live chips) and D3 (preview), owner, 2026-10-28.

### D2. Rows a phone has are marked in the sidebar

`SessionInfo` gains `phone: Option<String>`, the name of the paired phone in use for the session,
or `None`. The host sends the session again whenever it changes. The sidebar row shows a phone mark
after its title, with the tooltip "Open on <phone>", and hides its age. The status word and glyph
stay, so a working session still says so.

Rejected: only marking the shown session (the owner looks for the flag in the sidebar first).

### D3. Only an explicit act takes the session back on the desktop

The desktop takes the session back on "Use here", Enter in the page, or opening the session from
the sidebar (an attach). Its window becoming active does not, because glancing at the Mac would then
throw the phone out of the session. Typing anywhere else in the app does not either.

### D4. The phone closes to home when it loses the session

The phone stays the device that takes a session on opening it (no ask). When the host says another
client has the open session, the phone leaves the session or shell screen for home and shows a note
for a few seconds: "<session> is open on <computer>". The cover and its "Use here" are removed.

## Risks

- Opening a session from the desktop's sidebar now closes it on the phone, which is the intent,
  but a desktop restart that reopens the last session would do the same. Mitigation: the app only
  reopens a session at start when it is not held by a phone (`SessionInfo.phone`).
- An unsent draft on the phone is lost when the screen closes. Accepted: the desktop took over.

## Open questions

- None.
