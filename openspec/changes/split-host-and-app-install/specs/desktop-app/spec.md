## ADDED Requirements

### Requirement: Finding this computer's host
The app SHALL use this computer's host only when the installer recorded one or `~/.local/bin/hq` exists, starting it when it is not running, and SHALL otherwise open at Connect to a Host and never start a host.

#### Scenario: App-only install
- **WHEN** the owner opens the app on a laptop with an app-only install
- **THEN** the app shows Connect to a Host, and the host switcher has no entry for this computer

#### Scenario: Full install
- **WHEN** the owner opens the app on a Mac with a full install and the host is not running
- **THEN** the app starts the host from `~/.local/bin/hq` and shows its sessions

### Requirement: Updates follow the installed parts
The app's updater SHALL install the same parts the installer recorded: the host and the app for a full install, the app only for an app-only install.

#### Scenario: Updating an app-only laptop
- **WHEN** the owner installs an update on an app-only laptop
- **THEN** only the app is updated and restarted, and no host is installed
