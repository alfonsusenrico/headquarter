## MODIFIED Requirements

### Requirement: Unread completion
The system SHALL mark a session idle not read yet when its agent completes a turn, and SHALL mark it idle only after the owner has seen it. An agent that stops without completing a turn, reporting idle rather than done, SHALL leave the session idle.

#### Scenario: Turn completes unattended
- **WHEN** the agent reports that a turn completed
- **THEN** the session's status becomes idle not read yet

#### Scenario: Owner looks at the session
- **WHEN** a session is idle not read yet and the owner interacts with it
- **THEN** the session's status becomes idle

#### Scenario: Codex starts a new conversation
- **WHEN** the owner has read a Codex session and types `/new`, and Codex reports working, then idle
- **THEN** the session's status is idle, not idle not read yet
