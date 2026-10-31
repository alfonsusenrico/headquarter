## ADDED Requirements

### Requirement: Remote control runs while a phone is paired
The system SHALL run the remote control endpoint only while at least one phone is paired or a pairing code is shown, SHALL keep the host running while it runs, and SHALL stop it and close every phone connection when the last phone is removed.

#### Scenario: No phone paired
- **WHEN** no phone is paired and no pairing code is shown
- **THEN** the host opens no network endpoint

#### Scenario: Pairing the first phone
- **WHEN** no phone is paired and the owner shows a pairing code
- **THEN** the host starts the endpoint, and after a phone pairs it keeps running

#### Scenario: A code expires unused
- **WHEN** no phone is paired and the shown code expires or is cancelled
- **THEN** the endpoint stops

#### Scenario: The last phone removed
- **WHEN** the owner removes the only paired phone while it is connected
- **THEN** its connection closes and the host opens no network endpoint

## REMOVED Requirements

### Requirement: Remote control off until turned on
**Reason**: Remote control follows the paired phones (owner, 2026-10-31); there is no switch.
**Migration**: See "Remote control runs while a phone is paired". `hq remote on` and `off` are removed.

## MODIFIED Requirements

### Requirement: Remote Control window
The system SHALL offer, from the app's menu, a Remote Control window with how the computer is reached while remote control runs, a Pair a Phone button that shows the pairing code as an image QR code with its remaining time and as text, and the paired phones with whether each is connected or when it was last seen and a Remove button. It SHALL have no switch for remote control.

#### Scenario: Pairing from the window
- **WHEN** the owner opens Headquarter ▸ Remote Control…, presses Pair a Phone and scans the QR code with a phone
- **THEN** the window says the phone is paired and lists it as connected

#### Scenario: Turned on from the window
- **WHEN** no phone is paired and the owner presses Pair a Phone
- **THEN** the host starts its endpoint for the code, and the window says how the computer is reached

#### Scenario: Terminal too small for the QR code
- **WHEN** the owner runs `hq remote pair` in a terminal smaller than the QR code
- **THEN** the command names the size it needs and shows the code as text instead of a broken QR code

### Requirement: Getting the phone app
The Remote Control window SHALL always show, as its first step, a QR code and a Copy link button for the newest stable Headquarter for Android APK, through an address that always names the latest release and skips pre-releases.

#### Scenario: Scanning for the app
- **WHEN** the owner opens Headquarter ▸ Remote Control… with no phone paired and scans the first QR code with the phone's camera
- **THEN** the phone downloads `headquarter-android.apk` from the latest stable release

#### Scenario: Copying the link
- **WHEN** the owner presses Copy link
- **THEN** the clipboard holds the latest-download address and the button reads "Copied" for a moment

#### Scenario: Remote control off
- **WHEN** no phone is paired, so remote control is not running
- **THEN** the step still shows, so the app can be installed before pairing

#### Scenario: A phone already paired
- **WHEN** at least one phone is paired
- **THEN** the QR code and Copy link still show in full, with no Show button
