# Spec Delta

## MODIFIED Requirements

### Requirement: One-line install
The system SHALL install the command line and the app for the owner's operating system and architecture with one command, without administrator rights, and SHALL say how to put the install folder on `PATH` when it is missing.

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
