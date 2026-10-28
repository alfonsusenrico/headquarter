# Verification: getting the phone app from Remote Control

Tested: branch `feat/phone-app-download` from `main` @ 714be1b, working tree with the change.
2026-10-28.

## Commands

- `./scripts/check`: passed. Rust 541 tests in 27 suites, 0 failed. fmt, clippy `-D warnings`,
  cargo-deny, Android checks, OpenSpec strict 19/19. `pakem project check`: 0 errors, 2 old warnings.
- New tests: `the_phone_app_link_names_the_latest_stable_apk`,
  `the_app_step_folds_once_a_phone_is_paired`, `the_phone_app_link_fits_a_small_qr_code`.
- `curl -sI` on the link: 302 to `releases/download/v0.1.16/headquarter-android.apk`.

## On screen

macOS dev build on an empty test data folder, then the dev phone (realme RMX1921, debug build)
paired with that host. Captures: `research/evidence/phone-app-download/`.

- No phone paired, remote control off: "GET THE PHONE APP" first, above the switch, with the QR
  code, the two lines and Copy link.
- Remote control on: the order reads get the app, the switch, Pair a phone, Paired phones.
- The QR code read back from the window's captures with Core Image (dark and light): the
  latest-download link, exactly.
- Copy link: the clipboard held the link, and the button read "Copied", then "Copy link" again
  after 2 s.
- After pairing the dev phone: the step folded to "For another phone" with Show, and Show opened
  it. Reopening the window folds it again.
- Dark and light themes.
- The link opened on the phone (Brave) offered `headquarter-android.apk`, 36.35 MB, for download.
  The download was cancelled and no file was left on the phone.

Found and fixed while checking: the folded row repeated its heading ("Get the phone app" under
"GET THE PHONE APP"). It now reads "For another phone" and says what Show does.

Found, not part of this change: with Remote Control open, Quit's dialog opens underneath it.
Esc then cancels the hidden dialog instead of closing the window.

## Not verified

- Scanning the QR code with a phone's camera (the phone could not be pointed at the screen
  remotely). The code's content was read back from the screen instead.
- Installing the downloaded APK over the browser (the download was cancelled).
- Linux.

## Owner check

Pending.
