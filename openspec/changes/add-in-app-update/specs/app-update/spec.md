## ADDED Requirements

### Requirement: Checking for releases
The system SHALL check for a newer release of Headquarter at app start, every 6 hours while the app is open, and when the owner chooses Check for Updates…, counting only releases newer than the running version, and SHALL stay silent when an automatic check fails but say why when a manual one does.

#### Scenario: Newer release
- **WHEN** the app runs 0.1.8 and the latest release is 0.1.9
- **THEN** the sidebar shows "Update 0.1.9" at its bottom

#### Scenario: Up to date
- **WHEN** the owner chooses Check for Updates… and the latest release is the running version
- **THEN** the app says it is on the latest version, and no pill shows

#### Scenario: Offline
- **WHEN** an automatic check cannot reach GitHub
- **THEN** nothing is shown, and the next check runs as planned

### Requirement: Update notice
The system SHALL keep the update pill in the sidebar until the update is installed, and SHALL open from it a window naming the new version with Update Now and Later, where Later only closes the window.

#### Scenario: Later
- **WHEN** the owner opens the pill and presses Later
- **THEN** the window closes and the pill stays

#### Scenario: Not installed by the installer
- **WHEN** a newer release exists and the running app is not one the installer installed
- **THEN** the window names the version and offers the release page instead of Update Now

### Requirement: Installing an update
The system SHALL install an update with that release's own installer after checking it against the checksum GitHub publishes for it, SHALL show the steps download, verify, install and restart as they happen, and SHALL leave the installed version unchanged and say why when any step fails.

#### Scenario: Steps
- **WHEN** the owner presses Update Now with no session busy
- **THEN** the window shows download, verify and install completing in turn, then the restart

#### Scenario: Checksum differs
- **WHEN** the downloaded installer does not match its published checksum
- **THEN** it is not run, the window says so, and the installed version is unchanged

#### Scenario: Installer fails
- **WHEN** the installer exits with an error
- **THEN** the window shows its error and the installed version is unchanged

### Requirement: Busy sessions before a restart
The system SHALL, before restarting for an update while any live sidebar session is working or needs the owner, name those sessions and offer Restart Now or Wait, and with Wait SHALL install at once and restart by itself as soon as no session is busy.

#### Scenario: Ask first
- **WHEN** the owner presses Update Now while "Fix login" is working
- **THEN** the window names "Fix login", says restarting interrupts it, and offers Restart Now and Wait

#### Scenario: Wait
- **WHEN** the owner chooses Wait and "Fix login" finishes its turn
- **THEN** the new version is installed and the restart runs without another click

### Requirement: Restarting onto the new version
The system SHALL restart for an update by stopping the host, which marks its running sessions to start again, then reopening the app from the new version, whose host starts those sessions, the shown one first.

#### Scenario: Sessions come back
- **WHEN** an update restarts Headquarter with three sessions running and one inactive
- **THEN** the app reopens on the new version, the three start again on their conversations, and the inactive one stays inactive

### Requirement: Finishing an update made elsewhere
The system SHALL offer the update restart when the host it connects to runs an older version than the app.

#### Scenario: Installed with the install line
- **WHEN** the owner installed a newer version with the install line and reopens the app while the old host still runs
- **THEN** the sidebar shows "Restart to finish update", which offers the restart
