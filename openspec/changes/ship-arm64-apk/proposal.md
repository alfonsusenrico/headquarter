# The release APK ships for arm64 only

Status: approved by the owner on 2026-11-03 ("agree with all", the ci-speed report's
recommendation).

## Why

The APK job takes about 13 minutes, 8 of them building the Rust core four times in a row: for
`arm64-v8a`, `armeabi-v7a` and `x86_64`, then for the Kotlin binding
(`research/notes/2026-11-03-ci-speed-report.md`). Headquarter's phone app is for the owner's own
64-bit phone. `armeabi-v7a` serves only 32-bit phones, and `x86_64` mostly emulators.

## Expected Outcome

- The release APK carries the core for `arm64-v8a` only, and the APK job is about 4 minutes
  shorter.
- Debug builds keep all three ABIs, so an emulator still runs the app during development.

## What Changes

- `scripts/android-core` builds the ABIs named in `HQ_ANDROID_ABIS` (default: all three).
- The release build type packages `arm64-v8a` only; `build-android.yml` builds only that core.
- The `release-install` spec names the ABI.

## Non-goals

- Building the ABIs in parallel or caching them.

## Capabilities

No new project capability. Modified: `release-install`.

## Impact

`scripts/android-core`, `mobile/android/app/build.gradle.kts`, `.github/workflows/build-android.yml`.
A 32-bit phone can no longer install release APKs.
