## ADDED Requirements

### Requirement: The folder field starts empty
The New session window's folder field SHALL start empty, with a hint that an empty field means the home folder, and SHALL list and start in the home folder while it is empty.

#### Scenario: Typing a folder at once
- **WHEN** the owner presses Cmd-N and types `~/code/api/`
- **THEN** the field reads `~/code/api/` with nothing to delete first

#### Scenario: Starting in the home folder
- **WHEN** the owner presses Cmd-N, leaves the field empty and presses Enter on "Open here"
- **THEN** a session starts in the home folder
