# Verification: fix-phone-scroll-link

Revision: `fix/phone-scroll-link` b78b204, on `main` 1e4256e.

| Check | Result |
| --- | --- |
| `./scripts/check` | passed: 666 tests (1 ignored), cargo-deny, OpenSpec 19/19 strict |
| `./scripts/android-core`, ktlint, Android Lint, debug APK | passed |
| Unit tests (phone core) | a command racing a frame delivered 7 bytes at a time loses nothing (the old `read_frame` in `select!` fails the same test with `TooLarge(117901063)`); one scroll batch on its way, the next on its answer or after 2 s, at most 10 reports, one batch kept, opposite drags cancel, history scrolls at once; one log line per ending with no content |
| Host tests | a late viewer of a full-screen agent gets its modes and no old frames; an inline agent's history comes again; attaching at the agent's own size still repaints |

Reproduction on this Mac (stand-in phone on the real core, stand-in agent repainting for every
wheel report, n0's relay forced with a local switch never committed;
`research/evidence/fix-phone-scroll-link/summary.md`):

- Before: the link was lost three times in a 10 s drag (`read: frame of 993085467 bytes exceeds
  1048576`); with frames read whole but no batching, the screen was 15.5 s behind; a reopen showed
  old frames for many seconds.
- After: no link lost; 1.5 s behind after the drag (polled every 0.5 s); a reopen with a 200x50
  viewer holding the session showed the current frame at the phone's size within 0.5 s.

The owner's phone, 2026-11-16 (TECNO CN7c, this branch's debug build beside the release app, paired
as a second device; the owner's real host still on `0.4.0-rc.2`, so the host-side D6 was not in
play; a fullscreen Claude Code session at work, "brand-studio-agent"; only scrolling, nothing
typed):

- Mobile data (5G), relayed: 138 drags in 60 s, no link lost on the phone or the host. The screen
  held its place within half a second of the last drag.
- Wi-Fi (same subnet as the Mac): the link moved over without reconnecting; 138 drags in 60 s, no
  link lost, the screen current right after.
- Opening the session showed the old, wider frames for about 25 s (the replay through the relay),
  as expected with the host not yet updated: D6 removes it, verified in the reproduction only.
- Phone log (`owner-phone-logcat.txt`): one `link connected: path relayed`, nothing else.

Findings for 1.4 (direct on Wi-Fi):

- The host logs a connection's path once, as it connects (`host/remote.rs`, `path_kind`), when
  iroh still uses the relay; a later switch to direct is never logged. "All relayed" in the owner's
  log does not show the phone never went direct. Follow-up, not in this change: log path changes.
- The Mac's home relay is n0's `usw1-1` (US West), so a relayed byte between the phone and the Mac
  in Indonesia crosses the Pacific twice.

Not verified:
- D6 on the owner's own host: after the release, with the host updated.
- Real Claude Code's repaint bytes per wheel report were not measured; the phone scrolled smoothly.
