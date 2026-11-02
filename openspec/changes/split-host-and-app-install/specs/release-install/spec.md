## MODIFIED Requirements

### Requirement: One-line install
The system SHALL install, with one command and without administrator rights, the host and the app (default), the host only (`--host-only`), or the app only (`--app-only`), for the owner's system and architecture, and SHALL say how to put the install folder on `PATH` when it is missing.

#### Scenario: macOS
- **WHEN** the owner runs the install line on an Apple silicon Mac
- **THEN** `hq` and `headquarter` are in `~/.local/bin` and `Headquarter.app` is in `~/Applications`, opening without a security prompt

#### Scenario: Linux
- **WHEN** the owner runs the install line on x86-64 Linux
- **THEN** `hq`, `headquarter` and `headquarter-app` are in `~/.local/bin`, Headquarter appears in the desktop's application menu, and any missing system library is named with the package that provides it

#### Scenario: Folder not on PATH
- **WHEN** `~/.local/bin` is not on `PATH`
- **THEN** the installer prints the line to add to the shell's start-up file

#### Scenario: Linux window matched to its entry
- **WHEN** Headquarter runs on a Linux desktop after the install line
- **THEN** its window carries the app id `headquarter`, the desktop entry names the same window class, and the dock and window switcher show the Headquarter icon

#### Scenario: Host only on a server
- **WHEN** the owner runs the install line with `--host-only` on a headless Linux server
- **THEN** `hq` and `headquarter` are installed, no app or menu entry is, and the host runs as the owner's service

#### Scenario: App only
- **WHEN** the owner runs the install line with `--app-only` on a Mac
- **THEN** only `Headquarter.app` is installed, and it opens at Connect to a Host

## ADDED Requirements

### Requirement: Install parts are remembered
The installer SHALL record which parts it installed, keep them when run again without an option, change them when run with another option, and remove only recorded parts on uninstall.

#### Scenario: Rerun on the server
- **WHEN** the owner reruns the plain install line on a host-only server
- **THEN** the host is updated and no app is installed

#### Scenario: From full to app only
- **WHEN** the owner runs the install line with `--app-only` on a full install
- **THEN** the app stays, `hq` and the host service are removed, and the data folders remain and are named

### Requirement: App archives carry no host
The app archives SHALL hold the desktop app only, and the macOS app bundle SHALL not contain `hq`.

#### Scenario: Release assets
- **WHEN** a release is published
- **THEN** the app archives contain no `hq` executable, and the host archives contain `hq` and `headquarter`
