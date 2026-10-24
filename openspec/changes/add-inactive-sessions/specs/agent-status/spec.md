## MODIFIED Requirements

### Requirement: Five session statuses
The system SHALL report every launched session as exactly one of: working, needs you, idle not read yet, idle (stored as working, needs-you, done-unread, done). A session whose agent is not running is stored as inactive and shown as inactive, or as starting while it waits to start with the host. When signals conflict, needs you SHALL take precedence over working, working over idle not read yet, and idle not read yet over idle.

#### Scenario: Permission request while working
- **WHEN** a session is working and the agent reports a permission request
- **THEN** the session's status becomes needs you

#### Scenario: Owner answers the request
- **WHEN** a session is needs you and the agent reports tool activity or a new prompt
- **THEN** the session's status becomes working

#### Scenario: Words shown
- **WHEN** a session's agent finishes a turn the owner has not seen
- **THEN** the sidebar, the rail tooltip and `hq ls` say "idle, not read yet", and `hq ls --json` still reports `done-unread`

#### Scenario: Agent not running
- **WHEN** a sidebar session's agent was quit and the host is not about to start it
- **THEN** the sidebar shows it as inactive and `hq ls` says "inactive"

## ADDED Requirements

### Requirement: Inactive sessions
The system SHALL keep a sidebar session whose agent ends without the owner closing it (a quit from inside the agent, or an exit with an error) in the sidebar as inactive, SHALL keep a running session idle however long it waits, and SHALL start an inactive session's agent again only when the owner asks.

#### Scenario: Quit from inside
- **WHEN** the owner types `/exit` in a Claude Code session
- **THEN** the session stays in the sidebar as inactive and its agent's memory is freed

#### Scenario: Agent crashed
- **WHEN** a hosted agent exits with a non-zero code without the owner closing it
- **THEN** the session stays in the sidebar as inactive and is not restarted

#### Scenario: Waiting overnight
- **WHEN** a hosted session has been idle for twelve hours with its agent still running
- **THEN** its status is still idle

## REMOVED Requirements

### Requirement: Stopped sessions are idle
**Reason**: Replaced by "Inactive sessions": a stopped session is now shown as inactive and starts only when the owner asks, or with the host when it was running (owner, 2026-10-24).
**Migration**: None for data; migration `0008` sets which stopped sessions start with the next host.
