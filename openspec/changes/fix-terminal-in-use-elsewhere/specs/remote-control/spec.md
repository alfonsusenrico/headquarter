## ADDED Requirements

### Requirement: The host names the phone
The system SHALL name a paired phone's connection to the host with that phone's name from the index, and SHALL refuse a phone's own request to name its connection, keeping the connection.

#### Scenario: Named by the host
- **WHEN** a phone paired as "Pixel 8" opens a session that the desktop also shows
- **THEN** the desktop is told that "Pixel 8" is in use

#### Scenario: A phone names itself
- **WHEN** a phone sends a request naming its own connection
- **THEN** the request is refused, the connection stays open, and the name is unchanged
