# Verification: ship-arm64-apk

## Revision tested

Branch `ci/android-arm64-release` on `main` at `2875d7f` (v0.2.0-alpha.2), the owner's MacBook
Air, JDK 17, NDK 30.0.16248370.

## Checks

| Check | Result |
| --- | --- |
| `./scripts/check` (Rust, ktlint, Android Lint with warnings as errors, debug and release APKs, OpenSpec) | passed: 611 tests, 0 failed |
| `HQ_ANDROID_ABIS=arm64-v8a ./gradlew assembleRelease` | the APK's native libraries are all under `lib/arm64-v8a/` (the core, JNA, AndroidX); 34 s with the core already built |
| `./gradlew assembleDebug` (default ABIs) | `libhq_mobile.so` and `libjnidispatch.so` for `arm64-v8a`, `armeabi-v7a` and `x86_64` |
| `HQ_ANDROID_ABIS="arm64-v8a mips" scripts/android-core` | refused: "unknown ABI mips", exit 1 |

Android Lint's `ChromeOsAbiSupport` (x86_64 Chromebooks) is suppressed on the release filter with
the reason beside it: not serving them is this change's decision.

## Not verified

- The release job's time and the published APK: the next release tag (expected: about 4 minutes
  less than 785 s).
