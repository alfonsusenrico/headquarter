## ADDED Requirements

### Requirement: Clients see every link
The system SHALL report to a client every outside session linked to a conversation
while that client connects, so the client shows the session as running in another
terminal without waiting for the session's status to change.

#### Scenario: Idle session linked while the app opens
- **WHEN** the app opens and the outside scan links an idle session in another terminal after the app listed the sessions
- **THEN** the app shows that session as running in another terminal and does not offer to add it

#### Scenario: Change before an answer
- **WHEN** the host reports a change before it answers the client's list or usage request
- **THEN** the client keeps that change and applies it after the list, in the order received
