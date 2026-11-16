# Tasks

## 1. Diagnose

- [x] 1.1 The phone logs why its link ended (D1). Evidence: unit test of the log line for each
  ending, with no content in it.
- [x] 1.2 Reproduction with the stand-in phone and a stand-in full-screen agent over n0's relay
  (D2). Evidence: `research/evidence/fix-phone-scroll-link/`: wheel reports sent, output bytes per
  second, catch-up time, how the link ended.
- [ ] 1.3 With the owner's go: the owner's phone with `adb logcat` on Wi-Fi and mobile data (D3),
  folded into 3.2 since Phase 1 confirmed the causes without it. Evidence: the log lines, saved
  without personal data.
- [ ] 1.4 Why the phone is never direct on Wi-Fi, during 3.2 on the owner's phone. Evidence: a note
  in `research/evidence/`.
- [ ] 1.5 Write the confirmed cause and the chosen fix into `design.md` (Phase 2) and show the
  owner before building it.

Expected Result: the cause of the dropped links is known and recorded, and the fix is agreed.

## 2. Fix

- [ ] 2.1 D4, frames read in their own task. Evidence: a unit test that a command arriving while a
  frame is half delivered loses no bytes; the reproduction relayed keeps the link for a minute of
  dragging.
- [ ] 2.2 D5, one scroll batch in flight. Evidence: unit tests of the batching (lines kept while a
  batch is out, sent on output or after 300 ms, at most 10 reports); the reproduction relayed is
  current within a second of the drag ending.
- [ ] 2.3 D6, a full-screen reattach without old frames. Evidence: a host test that an attach on
  the alternate screen gets the modes but no catch-up bytes, and an inline one keeps them; the
  reproduction with a 200x50 viewer shows the phone's layout within a second.

Expected Result: a minute of scrolling keeps the link, and a reconnect shows the phone's layout.

## 3. Verification and delivery

- [ ] 3.1 `./scripts/check`, `./scripts/android-core`, the Android build and Lint; the Windows check
  runs on the pull request.
- [ ] 3.2 With the owner's go: a debug APK on the owner's phone, a minute of scrolling a fullscreen
  Claude Code session on Wi-Fi and on mobile data.
- [ ] 3.3 After the owner accepts: archive, push `fix/phone-scroll-link`, open the PR, report its
  URL.
