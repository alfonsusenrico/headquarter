# Tasks

## 1. Remote Control window

- [ ] 1.1 The link, built from the repository address the update check keeps (D1). Evidence: unit test of the address, `curl -sI` showing the 302 to the current APK.
- [ ] 1.2 The step with its QR code, text and Copy link, folding once a phone is paired (D2, D3, D4). Evidence: unit test of when the step is open, the dev app on screen in both themes with no phone paired and with one, the QR code scanned by the dev phone's camera opening the APK download, Copy link pasted.
- [ ] 1.3 `AGENTS.md` `## Release` Verify: the latest-download link answers with the new APK. Evidence: the diff.

Expected Result: the owner gets the app onto a phone from the Remote Control window.

## 2. Verification and delivery

- [ ] 2.1 `./scripts/check` passes. `verification.md` written.
- [ ] 2.2 The owner checks it.
- [ ] 2.3 After the owner accepts: archive, push `feat/phone-app-download`, open the PR, report its URL.

Expected Result: one PR with the download step.
