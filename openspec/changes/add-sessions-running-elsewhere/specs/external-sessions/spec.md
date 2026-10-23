# Spec Delta

## MODIFIED Requirements

### Requirement: Clients see every link
The system SHALL report to a client every outside session linked to a conversation
while that client connects, so the client shows the session as running in another
terminal without waiting for the session's status to change, and SHALL report each
outside agent with no known conversation as it appears, changes, and ends or links,
so a client never keeps one that is gone.

#### Scenario: Idle session linked while the app opens
- **WHEN** the app opens and the outside scan links an idle session in another terminal after the app listed the sessions
- **THEN** the app shows that session as running in another terminal and does not offer to add it

#### Scenario: Change before an answer
- **WHEN** the host reports a change before it answers the client's list or usage request
- **THEN** the client keeps that change and applies it after the list, in the order received

#### Scenario: Unlinked agent links
- **WHEN** an outside Claude session is first listed with no known conversation and its conversation is imported a moment later
- **THEN** the app drops the "claude pid N" entry and shows the linked session in its place, pickable

#### Scenario: Unlinked agent starts or exits while the app is open
- **WHEN** an agent with no known conversation starts in another terminal, and later exits, while the app is open
- **THEN** its entry appears in Add sessions within one outside scan, and disappears within one outside scan after it exits
