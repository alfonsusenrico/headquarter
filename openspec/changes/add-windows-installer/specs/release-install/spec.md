## MODIFIED Requirements

### Requirement: Releases from CI
The system SHALL build and publish releases only from CI on version tags: the host and command line for macOS and Linux on both architectures, the app for those, and for Windows x86-64 a setup program, with checksums and the installer.

#### Scenario: Tag pushed
- **WHEN** the owner pushes the tag `v0.1.0`
- **THEN** CI builds the archives for the four Unix targets and the Windows setup, and publishes them with checksums and `install.sh` on the `v0.1.0` release

## ADDED Requirements

### Requirement: Windows setup
The release SHALL offer the Windows app as `headquarter-setup-x86_64.exe`: a setup wizard that installs for the current user into `%LOCALAPPDATA%\Programs\Headquarter` without an administrator prompt, adds a Start menu entry and optionally a desktop shortcut, registers an uninstaller, and updates an installed copy in place. Uninstalling SHALL keep the app's data.

#### Scenario: First install
- **WHEN** the owner runs the setup on a PC without Headquarter and accepts the defaults
- **THEN** no administrator prompt appears, the app is in `%LOCALAPPDATA%\Programs\Headquarter`, the Start menu lists Headquarter with its icon, and Finish can open it

#### Scenario: Updating
- **WHEN** the app is running and the owner runs a newer setup
- **THEN** the setup closes the app, replaces it in place, and the paired hosts and settings are kept

#### Scenario: Uninstalling
- **WHEN** the owner removes Headquarter in Settings ▸ Apps
- **THEN** the program and its shortcuts are removed and `%APPDATA%\Headquarter` stays

#### Scenario: Checksums
- **WHEN** the release is published
- **THEN** `headquarter-setup-x86_64.exe.sha256` matches the setup, and the release carries no Windows zip
