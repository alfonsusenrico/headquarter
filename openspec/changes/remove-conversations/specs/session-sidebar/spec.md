# Spec Delta

## ADDED Requirements

### Requirement: Removing past conversations
The system SHALL offer, on each Add sessions row whose conversation is not running, a menu to remove it from its agent the way that agent allows: Claude Code's transcript and the folder beside it to the system Trash, Codex through its own archive or permanent delete, and nothing for agy; SHALL let Move to Trash and Archive be undone for 5 seconds before they take effect, SHALL ask before Delete permanently, and SHALL keep the menu's space on rows without one so their ages line up.

#### Scenario: Claude conversation to the Trash
- **WHEN** the owner chooses Move to Trash on a past Claude Code conversation and lets the toast pass
- **THEN** its transcript and the folder beside it are in the Trash, and the row is gone from Add sessions

#### Scenario: Undo
- **WHEN** the owner chooses Archive on a Codex conversation and presses Undo within 5 seconds
- **THEN** the row comes back and Codex is not called

#### Scenario: Permanent delete
- **WHEN** the owner chooses Delete permanently on a Codex conversation
- **THEN** a confirmation says it cannot be undone, and only after Delete does Headquarter run `codex delete --force` with its id

#### Scenario: agy and running sessions
- **WHEN** a row is an agy conversation, or a session running here or in another terminal
- **THEN** it has no menu, and its age lines up with the other rows

#### Scenario: Agent refuses
- **WHEN** Codex fails to archive or delete
- **THEN** the row comes back with a note that it could not be removed, and the host log names the agent and the way only
