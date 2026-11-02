## ADDED Requirements

### Requirement: The app on Windows
On Windows the app SHALL be a client of remote hosts only: it SHALL open at Connect to a Host until a host is paired, keep its data in `%APPDATA%\Headquarter`, and never look for or start a host on the PC.

#### Scenario: First launch on Windows
- **WHEN** the owner opens the app on Windows for the first time
- **THEN** it shows Connect to a Host, and after a valid code it shows that host's sessions

#### Scenario: No local host
- **WHEN** the owner opens the host switcher on Windows
- **THEN** it lists only paired hosts and Connect to a Host

### Requirement: Updates on Windows
On Windows a newer release SHALL show the update notice with a link to its release page instead of installing it.

#### Scenario: Newer release
- **WHEN** a newer release exists and the app on Windows checks for updates
- **THEN** the notice names the version and opens its release page when chosen
