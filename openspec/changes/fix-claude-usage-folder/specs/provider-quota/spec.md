## ADDED Requirements

### Requirement: The hidden Claude read picks a trusted folder
The system SHALL run the hidden Claude usage read in the most recently used session folder that Claude Code records as trusted, reading only that record and never changing it, and SHALL fall back to the most recently used existing folder when the record cannot be read.

#### Scenario: Working in the home folder
- **WHEN** the owner's most recent Claude Code session runs in the home folder, which Claude Code does not keep trusted, and an earlier one ran in a trusted project folder
- **THEN** the hidden read runs in that project folder and the usage updates

#### Scenario: No trusted folder
- **WHEN** none of the sessions' folders is trusted by Claude Code
- **THEN** no hidden session starts and the details say Claude Code needs to be opened once in a project folder

### Requirement: Refresh after a failed reading
A refresh SHALL read an agent again at once when its last reading failed, even within the refresh floor.

#### Scenario: Refresh twice after a failure
- **WHEN** a Claude reading fails and the owner presses Refresh 10 seconds later
- **THEN** a new reading is taken and the button shows "Refreshing…" until it is in
