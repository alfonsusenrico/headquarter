## MODIFIED Requirements

### Requirement: Android APK in releases
The system SHALL publish with every version tag a signed Android APK named `headquarter-android.apk`, for arm64-v8a, with its SHA-256 in `headquarter-android.apk.sha256`, built and signed by the same release run as the desktop archives, and SHALL publish nothing when the APK cannot be built or signed.

#### Scenario: A release
- **WHEN** the owner pushes a version tag
- **THEN** the release carries `headquarter-android.apk` and its `.sha256` beside the desktop archives, `apksigner verify` accepts the APK, and its native libraries are for `arm64-v8a` only

#### Scenario: Installing over the previous release
- **WHEN** a phone with the previous release's APK installs the new one
- **THEN** it installs as an update and keeps the paired computers

#### Scenario: The APK fails
- **WHEN** the Android build or its signing fails in a release run
- **THEN** no GitHub release is published for that tag

#### Scenario: Debug builds for an emulator
- **WHEN** the owner builds the debug app
- **THEN** it carries the core for `arm64-v8a`, `armeabi-v7a` and `x86_64`
