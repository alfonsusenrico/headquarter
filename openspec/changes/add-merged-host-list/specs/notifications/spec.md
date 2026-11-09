## ADDED Requirements

### Requirement: Notifications from every host
The app SHALL notify about sessions of every connected host, by the same rules as for this computer's, and SHALL name the host in a notification about a session on another computer. Clicking it SHALL show that session.

#### Scenario: The server's agent needs the owner
- **WHEN** the health agent on the server asks for permission while the owner works in another app on the Mac
- **THEN** a notification names the session and "server", and clicking it shows the session's terminal
