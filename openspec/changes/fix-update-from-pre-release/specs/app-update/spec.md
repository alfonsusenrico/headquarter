## MODIFIED Requirements

### Requirement: Pre-release builds
A build with a pre-release version SHALL run and check for updates as a version just before the release it leads to, so that release is offered to it once published, and SHALL never be offered a pre-release as an update.

#### Scenario: A release candidate before its release
- **WHEN** the app is a `0.1.14-rc.1` build and the latest release is `0.1.13`
- **THEN** the app starts normally and says Headquarter 0.1.14-rc.1 is the latest version

#### Scenario: A release candidate checks for updates
- **WHEN** the app is a `0.4.0-rc.2` build and the latest release is `0.4.0`
- **THEN** the app offers `0.4.0`, and What's new lists 0.4.0's notes

#### Scenario: A newer release
- **WHEN** the app is a `0.1.14-rc.1` build and the latest release is `0.1.15`
- **THEN** the app offers `0.1.15`

### Requirement: Finishing an update made elsewhere
The system SHALL offer the update restart when the host it connects to runs an older version than the app, where a pre-release is older than its own release.

#### Scenario: Installed with the install line
- **WHEN** the owner installed a newer version with the install line and reopens the app while the old host still runs
- **THEN** the sidebar shows "Restart to finish update", which offers the restart

#### Scenario: The host on the release candidate
- **WHEN** the app is `0.4.0` and this computer's host still runs `0.4.0-rc.2`
- **THEN** the sidebar shows "Restart to finish update"
