# Design: keep the phone's link up while scrolling a full-screen agent

## Context

- `crates/hq-mobile/src/terminal.rs` `scroll`: when the agent asked for SGR mouse reports, a drag
  of N lines sends `min(N, 10)` wheel reports at once. Without them, arrow keys on the alternate
  screen, else the phone's own history.
- `crates/hq-mobile/src/link.rs` `serve`: one loop reads frames and writes commands. Any read or
  write error, a frame that fails to parse, or an output frame from the phone ends the loop as lost.
  Every ending closes the connection with `bye`, and a lost link retries after a short wait.
- `crates/hq-mobile/src/lib.rs`: output is parsed in `Shared::output` under the shared lock that
  the screen's reads also take.
- `crates/hq-daemon/src/session.rs` `attach`: sends the catch-up buffer (up to 1 MB, 64 KB frames),
  then makes the new viewer the one in use, which resizes the PTY.
- `crates/hq-daemon/src/host/remote.rs` `proxy`: frames to the device go through a channel of 64.

## Phase 1 decisions (diagnosis)

### D1. The phone logs why its link ended

One line per ended link: lost (read error kind, write error kind, refused frame), stopped, or a
final status, with the path (relayed or direct) and how long it was connected. No session ids,
names, input or output. This matches the rule that the phone's logs carry connection events only
(AGENTS.md).

### D2. A reproduction without a real phone

A stand-in agent that asks for SGR mouse reports and repaints a full screen of text on every wheel
report, as Claude Code's fullscreen mode does. The stand-in phone drags it at a finger's pace,
over n0's relay, while a desktop client views the same session at a wider size. Measured: wheel
reports sent, output bytes received per second of dragging, time until the screen is current
again, and whether and how the link ends.

### D3. The owner's phone, with the owner's go

`adb logcat` filtered to the app's tag while the owner scrolls a fullscreen Claude Code session,
on Wi-Fi and then on mobile data. A capture with a personal notification is deleted. No PIN is
entered.

## Phase 2

Written here from Phase 1's evidence and shown to the owner before it is built. Candidates are in
the proposal.

## Risks

- The cause may be in iroh or the relay under load rather than in Headquarter. Then the fix lowers
  the load (fewer redraws), and the finding goes to `research/evidence/`.
