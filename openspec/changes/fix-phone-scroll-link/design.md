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

## Phase 1 findings (2026-11-16)

Reproduced with the stand-in phone and a stand-in full-screen agent over n0's relay
(`research/evidence/fix-phone-scroll-link/summary.md`):

1. Dropped links: `Session::serve` races `read_frame` against commands in `tokio::select!`.
   `read_frame` is not cancellation-safe (two `read_exact`), so a command arriving mid-frame drops
   bytes and the next header is read from payload: `read: frame of 993085467 bytes exceeds
   1048576`, three times in a 10 s drag. Reading frames in a task of their own kept the link
   through the same drags.
2. Freeze: about 100 KB/s of repaints for a finger-paced drag against about 42 KB/s through the
   relay: the screen was 15.5 s behind after a 10 s drag (0.5 s direct).
3. Rescaled look: a reattach replays up to 1 MB of old frames, some drawn for another viewer's
   size, before the agent repaints for the phone.

## Phase 2 decisions (proposed, for the owner's approval)

### D4. Frames are read whole, in a task of their own

The serving loop waits on a channel the reader task fills, which is cancellation-safe. Setup
(`ask_now`) reads as now, before the task starts. Rejected: making `read_frame` resumable (a
buffered reader kept across `select!` rounds): more code in a shared crate for the same result.

### D5. One scroll batch in flight

The core adds a drag's lines to a pending count (at most one batch, 10 lines, kept) and sends them
as one batch of wheel reports (at most 10) only when the previous batch has been answered. Arrow
keys on the alternate screen without mouse reports follow the same rule. Over a fast link that is
every frame. Over a slow relay the scroll takes bigger steps instead of queueing repaints, and stops
one answer after the finger.

"Answered" is the answer to a `Usage` request sent right behind each batch (2 s at most, should it
never come). The computer answers requests in order behind the output it has already queued for
the phone, so the answer arrives only once the output from before the batch has. `Usage` is the
cheapest request every computer answers with no side effect beyond its own interval-limited
Codex read. Changed while building (2026-11-16) from "output arrived, or 300 ms": during a relay
backlog old output keeps arriving, which would release a batch per frame and keep the backlog; and
from two batches kept to one, which brought the stand-in from 2.0 s to 1.5 s behind the finger.

Rejected: a fixed rate limit, which is too slow direct or too fast relayed. A new `Ping` request:
a host that does not know a request closes the connection, so it needs a protocol bump and every
computer, app and phone updated together.

### D6. A full-screen session reattaches without its old frames

When the session is on the alternate screen, the host sends the attaching viewer the terminal
modes (as now) but not the catch-up buffer, then resizes so the agent repaints for that viewer.
When the size does not change, the host's existing forced redraw (`session.rs`, `redraw`) makes
the agent repaint, so the viewer never waits on a blank screen. An inline session keeps the
replay, since its history is the phone's scrollback. Rejected: capping the
replay at a size, which still shows stale frames, only fewer.

### Not in this change

Why the phone is never direct on Wi-Fi: recorded in the evidence with the owner's phone, fixed
separately if it is Headquarter's.

## Risks

- The cause may be in iroh or the relay under load rather than in Headquarter. Then the fix lowers
  the load (fewer redraws), and the finding goes to `research/evidence/`.
