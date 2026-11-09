# Tasks

## 1. Build

- [x] 1.1 The banner (D1). Evidence: ktlint, Android Lint; on the phone below.
- [x] 1.2 `hq remote pair` colours and link (D2). Evidence: `the_qr_code_is_dark_on_light_in_a_terminal`;
  host test `hq_remote_pairs_lists_and_removes` checks the link and no colours when piped.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check`; `./gradlew ktlintCheck assembleDebug lint testDebugUnitTest`.
- [x] 2.2 The QR code from a dark terminal decoded (zxing-cpp, with and without inverted
  reading), and the banner on the phone. Evidence: `verification.md`.
- [ ] 2.3 After the owner accepts: archive, push `fix/remote-polish`, open the PR, report its URL.
