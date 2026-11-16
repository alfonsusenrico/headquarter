# Tasks

## 1. Diagnose

- [x] 1.1 The phone logs why its link ended (D1). Evidence: unit test of the log line for each
  ending, with no content in it.
- [ ] 1.2 Reproduction with the stand-in phone and a stand-in full-screen agent over n0's relay
  (D2). Evidence: `research/evidence/fix-phone-scroll-link/`: wheel reports sent, output bytes per
  second, catch-up time, how the link ended.
- [ ] 1.3 With the owner's go: the owner's phone with `adb logcat` on Wi-Fi and mobile data (D3).
  Evidence: the log lines, saved without personal data.
- [ ] 1.4 Why the phone is never direct on Wi-Fi. Evidence: a note in `research/evidence/`.
- [ ] 1.5 Write the confirmed cause and the chosen fix into `design.md` (Phase 2) and show the
  owner before building it.

Expected Result: the cause of the dropped links is known and recorded, and the fix is agreed.

## 2. Fix

- [ ] 2.1 Build the agreed fix. Evidence: unit tests named in the Phase 2 design; the reproduction
  (1.2) stays connected for a minute of dragging.
- [ ] 2.2 Reattach at the phone's size. Evidence: the reproduction with a wider desktop viewer; on
  screen through the stand-in phone's state.

Expected Result: a minute of scrolling keeps the link, and a reconnect shows the phone's layout.

## 3. Verification and delivery

- [ ] 3.1 `./scripts/check`, `./scripts/android-core`, the Android build and Lint; the Windows check
  runs on the pull request.
- [ ] 3.2 With the owner's go: a debug APK on the owner's phone, a minute of scrolling a fullscreen
  Claude Code session on Wi-Fi and on mobile data.
- [ ] 3.3 After the owner accepts: archive, push `fix/phone-scroll-link`, open the PR, report its
  URL.
