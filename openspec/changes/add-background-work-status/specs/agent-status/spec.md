## ADDED Requirements

### Requirement: Background work and reviews beside the status
The system SHALL show, beside a session's status, the work its agent still runs in the background after its turn (with its kind and count where the agent reports them) and anything the agent left for review, from what the agent itself reports or records. It SHALL show nothing extra when a signal is unknown or unreadable, and SHALL keep the five statuses' meaning.

#### Scenario: Claude background work after the turn
- **WHEN** a Claude Code session ends its turn while two background shell commands it started still run
- **THEN** the session's row and header say it has two shells running in the background

#### Scenario: agy artifact to review
- **WHEN** an agy turn ends with an artifact waiting for review
- **THEN** the session shows idle (or unread) with a review flag, and not needs you

#### Scenario: Signal unavailable
- **WHEN** the agent's record of its background work cannot be read
- **THEN** the session shows its status alone
