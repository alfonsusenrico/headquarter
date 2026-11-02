## MODIFIED Requirements

### Requirement: Releases from CI
The system SHALL build and publish releases only from CI on version tags: the host and command line for macOS and Linux on both architectures, the app for those and for Windows x86-64, with checksums and the installer.

#### Scenario: Tag pushed
- **WHEN** the owner pushes the tag `v0.1.0`
- **THEN** CI builds the archives for the four Unix targets and the Windows app zip, and publishes them with checksums and `install.sh` on the `v0.1.0` release

## ADDED Requirements

### Requirement: Windows checks on every pull request
The system SHALL build, lint with warnings as errors, and test the client crates for Windows on every pull request, on a free GitHub-hosted runner.

#### Scenario: A change breaks the Windows build
- **WHEN** a pull request adds code that does not compile for Windows
- **THEN** the Windows check fails on that pull request
