## MODIFIED Requirements

### Requirement: Shells outlive switching
The system SHALL keep a session's shell running while the owner switches to other sessions or restarts the app, SHALL show it again when the session is shown, and SHALL end it when it exits, when its session is closed, or when its session becomes inactive.

#### Scenario: Switching away
- **WHEN** a build runs in session A's shell and the owner opens session B, then A again
- **THEN** the build kept running and A's panel shows it

#### Scenario: Exit
- **WHEN** the owner types `exit` in the shell
- **THEN** the panel closes

#### Scenario: Session closed
- **WHEN** the owner closes a session whose shell is running
- **THEN** the shell ends with it, without another question

#### Scenario: Agent quit
- **WHEN** the owner quits the agent of a session whose shell is running
- **THEN** the session becomes inactive and its shell ends with the agent
