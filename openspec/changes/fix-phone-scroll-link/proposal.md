# Keep the phone's link up while scrolling a full-screen agent

Status: approved by the owner 2026-11-16 ("ok, approved, go ahead").

## Why

Over several days before 0.4.1, the owner's phone reconnected very often, mostly while scrolling a
session's terminal. The terminal froze, then showed the session as if rescaled, then reconnected.
The session was Claude Code in fullscreen mode. The Mac app was open, likely on the same session.
The phone was on Wi-Fi, sometimes on mobile data (owner, 2026-11-16).

Observed in this Mac's host log (2026-11-10 to 11-16):

- All 28 phone connections were relayed, none direct, also on Wi-Fi.
- Many ended with `closed by peer: bye` 2 to 80 seconds after connecting, and the phone dialled
  again about 2 seconds later. The phone sends `bye` whenever its link loop ends, and retries after
  about 2 seconds only for a link it considers lost. So the phone ended these links itself, after
  a read error, a write error or a frame it refused. It was not a QUIC timeout. The host logged
  nothing else at those moments.
- The phone logs nothing when a link ends, so the cause is not recorded.

Hypotheses from the code, none confirmed:

- H1: in a full-screen agent that asks for mouse reports, each drag step sends up to 10 wheel
  reports (`SCROLL_REPORTS`). Claude Code redraws its screen for each report, and the redraws come
  back over the relay. A few drags queue far more output than the relay carries.
- H2: the phone core parses that output under the same lock the screen reads from, so the screen
  freezes while it catches up.
- H3: on reattach the host replays up to 1 MB of output drawn at the last viewer's size (likely the
  Mac's wider window) before resizing the terminal to the phone, which looks rescaled until the
  agent redraws.
- H4: something under that load ends the phone's stream (a read or write error over the relay, or
  a frame the phone refuses). Unknown which.

## Expected Outcome

- Scrolling a full-screen Claude Code session on the phone, over the relay, keeps the link up and
  the terminal following the finger.
- After a reconnect, the phone shows the session at its own size, not another screen's layout.
- When the phone's link does end, the phone's log says why, without content.

## What Changes

Phase 1, diagnosis, before any fix:

- The phone core logs why each link ended (lost after a read error, a write error or a refused
  frame; stopped by the app; given up), and the path it used, as connection events only.
- Reproduce with the stand-in phone (`research/prototypes/phone-standin`, on the real core) against
  a sandbox session that asks for mouse reports and redraws its screen on every wheel report, over
  n0's relay. Measure the output bytes per drag, the time to drain them, and how the link ends.
- With the owner's go, the same on the owner's phone with `adb logcat` while scrolling. No PIN is
  entered.
- Find why the phone is never direct on Wi-Fi, and record it. A fix for that is a separate change.

Phase 2, the fix, chosen from the evidence and written into the design before it is built. The
likely candidates:

- Send at most one wheel report per drawn frame, with the lines it covers, instead of up to 10 per
  drag step.
- Parse output outside the lock the screen reads, so the screen never waits on a backlog.
- On reattach, resize to the phone before the replay, or replay only from the last full redraw.

## Non-goals

- The desktop app's scrolling.
- A personal relay (a setting, never the default, AGENTS.md).
- Direct connections on Wi-Fi beyond finding the reason.

## Capabilities

`android-app` and possibly `remote-control` (the reattach order). No new project capability.

## Impact

`crates/hq-mobile` (link, terminal, scroll), possibly `crates/hq-daemon/src/session.rs` (attach)
and the Kotlin terminal view. No protocol change expected. Ships in 0.4.2 with a new APK.
