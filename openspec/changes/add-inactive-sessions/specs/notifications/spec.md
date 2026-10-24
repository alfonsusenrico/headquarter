## ADDED Requirements

### Requirement: Start failures are notified
The system SHALL show one system notification for each sidebar session that could not start with the host, titled as the sidebar shows the session, giving the reason, and SHALL show it when the app next connects if the app was not running at the time.

#### Scenario: Folder gone
- **WHEN** the host starts and the folder of the sidebar session "Fix login" no longer exists
- **THEN** a notification titled "Fix login" reads "Couldn't start:" followed by the reason naming the missing folder, and the session is inactive

#### Scenario: Told once
- **WHEN** the owner has been notified about a start failure and the app restarts while the host keeps running
- **THEN** no second notification appears for it
