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
- **WHEN** the shown session's agent stopped before the app reopened
- **THEN** the session is shown with its details and "Open again", and its agent is not restarted

#### Scenario: Shown session left the sidebar
- **WHEN** the session last shown was closed or removed from the sidebar before the app reopened
- **THEN** the app opens without showing that session

#### Scenario: Display gone
- **WHEN** the last position was on an external display that is no longer connected
- **THEN** the window opens centred on the main display

### Requirement: Interrupted sessions are offered for resuming
The system SHALL mark sessions that were running when the host stopped without ending them, SHALL offer to resume them together, and SHALL NOT resume any session without the owner asking.

#### Scenario: After a reboot
- **WHEN** three sidebar sessions were running and the machine restarted
- **THEN** the sidebar notes that three sessions were running, with Resume all and Not now, and none of them is running yet

#### Scenario: Resume all
- **WHEN** the owner presses Resume all and one agent can no longer be found
- **THEN** the other two continue their conversations, and the note names the one that failed with the reason

#### Scenario: Not now
- **WHEN** the owner presses Not now
- **THEN** the note disappears and the sessions stay idle; opening one restarts it

#### Scenario: Ended on purpose
- **WHEN** the owner stopped everything with `hq stop` before restarting the machine
- **THEN** no session is offered for resuming

#### Scenario: Closed sessions
- **WHEN** the owner closed a session before the machine restarted
- **THEN** it is not counted in the note

### Requirement: Interrupted sessions from the command line
The system SHALL mark interrupted sessions in `hq ls` and resume them with `hq resume --interrupted`.

#### Scenario: hq resume --interrupted
- **WHEN** the owner runs `hq resume --interrupted` after a crash
- **THEN** each interrupted sidebar session is resumed without attaching, and each result is reported
