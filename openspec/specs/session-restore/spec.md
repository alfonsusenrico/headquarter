# session-restore Specification

## Purpose
How Headquarter picks up where the owner left off: the app reopens its window and the session last shown, and sessions a crash or restart interrupted are marked so the owner can resume them all or dismiss the note.

## Requirements

### Requirement: Window and view come back
The system SHALL reopen the app with the window's last size and position, when they fit a connected display, and SHALL show the session that was shown when it still exists and is still in the sidebar.

#### Scenario: Reopened
- **WHEN** the owner quits the app with the window on the right half of the screen showing session `api-fix`, and opens it again
- **THEN** the window opens on the right half showing `api-fix`

#### Scenario: Shown session stopped meanwhile
- **WHEN** the shown session became inactive before the app reopened
- **THEN** the session is shown with "Resume Session", and its agent is not started

#### Scenario: Shown session left the sidebar
- **WHEN** the session last shown was closed or removed from the sidebar before the app reopened
- **THEN** the app opens without showing that session

#### Scenario: Display gone
- **WHEN** the last position was on an external display that is no longer connected
- **THEN** the window opens centred on the main display
