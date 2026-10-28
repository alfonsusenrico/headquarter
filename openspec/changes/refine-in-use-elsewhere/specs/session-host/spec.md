## ADDED Requirements

### Requirement: The session list names the phone in use
The system SHALL include, with each session it lists or announces, the name of the paired phone that is the client in use for it, or none, and SHALL announce the session again whenever that changes.

#### Scenario: A phone takes a session
- **WHEN** the phone "Pixel 8" opens a session the desktop also shows
- **THEN** subscribers receive the session again naming "Pixel 8"

#### Scenario: The phone leaves
- **WHEN** the phone closes the session
- **THEN** subscribers receive the session again naming no phone
