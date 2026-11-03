# Tasks

## 1. Build

- [x] 1.1 `HQ_ANDROID_ABIS` in `scripts/android-core`; release packages `arm64-v8a` only; the
  release job builds only that core. Evidence: `assembleRelease` with `HQ_ANDROID_ABIS=arm64-v8a`
  and `assembleDebug` with the default; the native libraries listed in each APK.

## 2. Verification and delivery

- [x] 2.1 `./scripts/check` passes; `verification.md` written.
- [x] 2.2 The owner checks it. Accepted 2026-11-03 ("1 done").
- [x] 2.3 After the owner accepts: archive, push, open the PR. The next release tag proves the
  job's time and the APK's ABI.
