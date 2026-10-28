## ADDED Requirements

### Requirement: Getting the phone app
The Remote Control window SHALL show, as its first step and whether remote control is on or off, a QR code and a Copy link button for the newest stable Headquarter for Android APK, through an address that always names the latest release and skips pre-releases. Once a phone is paired, the step SHALL fold to one line that can be opened again.

#### Scenario: Scanning for the app
- **WHEN** the owner opens Headquarter ▸ Remote Control… with no phone paired and scans the first QR code with the phone's camera
- **THEN** the phone downloads `headquarter-android.apk` from the latest stable release

#### Scenario: Copying the link
- **WHEN** the owner presses Copy link
- **THEN** the clipboard holds the latest-download address and the button reads "Copied" for a moment

#### Scenario: Remote control off
- **WHEN** remote control is off
- **THEN** the step still shows, so the app can be installed before remote control is turned on

#### Scenario: A phone already paired
- **WHEN** at least one phone is paired
- **THEN** the step is one line with a Show button, and Show opens the QR code and Copy link
