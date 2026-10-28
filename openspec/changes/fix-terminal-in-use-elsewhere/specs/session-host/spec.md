## ADDED Requirements

### Requirement: Viewers told who is in use
The system SHALL tell each client attached to a session, or to its shell, which client is in use: itself, a client on the computer, or a paired phone by its name. It SHALL tell a client when it attaches, and SHALL tell every attached client whenever the client in use changes.

#### Scenario: The phone opens a session open on the desktop
- **WHEN** the desktop shows a session and the phone opens it
- **THEN** the phone is told it is in use, and the desktop is told the phone is in use, with the phone's name

#### Scenario: The desktop takes it back
- **WHEN** the owner then clicks in the desktop's terminal
- **THEN** the desktop is told it is in use, and the phone is told a client on the computer is in use

#### Scenario: The client in use leaves
- **WHEN** the phone in use closes the session and the desktop still shows it
- **THEN** the desktop is told it is in use
