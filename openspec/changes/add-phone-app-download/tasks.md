# Tasks

## 1. Remote Control window

- [x] 1.1 The link, built from the repository address the update check keeps (D1). Evidence: unit test of the address, `curl -sI` showing the 302 to the current APK. Done 2026-10-28: `phone_app_link` in `update.rs` beside `REPO_PAGE`, test `the_phone_app_link_names_the_latest_stable_apk`, `curl -sI` 302 to `releases/download/v0.1.16/headquarter-android.apk`.
- [x] 1.2 The step with its QR code, text and Copy link, folding once a phone is paired (D2, D3, D4). Evidence: unit test of when the step is open, the dev app on screen in both themes with no phone paired and with one, the QR code scanned by the dev phone's camera opening the APK download, Copy link pasted. Done 2026-10-28: tests `the_app_step_folds_once_a_phone_is_paired`, `the_phone_app_link_fits_a_small_qr_code`; on screen in both themes, open with no phone and remote off and on, folded after pairing the dev phone, Show. The QR code read back from the window captures (Core Image) gives the link, and the link opened on the phone offered `headquarter-android.apk` (36.35 MB) for download. The camera scan itself was not done, since the phone cannot be pointed at the screen remotely. Copy link: the clipboard held the link, the button read "Copied" for 2 s.
- [x] 1.3 `AGENTS.md` `## Release` Verify: the latest-download link answers with the new APK. Evidence: the diff. Done 2026-10-28 without an edit: the Verify step already checks that `releases/latest` is the new tag and that its assets include `headquarter-android.apk`, which is what GitHub resolves the link from, and a new line took `AGENTS.md` over pakem's 10,000-character limit.

Expected Result: the owner gets the app onto a phone from the Remote Control window.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes. `verification.md` written. Done 2026-10-28: 541 tests, 0 failed.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `feat/phone-app-download`, open the PR, report its URL.

Expected Result: one PR with the download step.
