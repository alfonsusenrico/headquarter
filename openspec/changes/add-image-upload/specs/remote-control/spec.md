## MODIFIED Requirements

### Requirement: What a phone may do
The system SHALL let a paired phone list and follow sessions, open and type into them, resize, resume inactive sessions, add past conversations, open a session's shell, mark sessions seen, read usage and send images to a session, and SHALL refuse every other request, keeping the connection.

#### Scenario: Typing
- **WHEN** a paired phone attaches to a session and sends text
- **THEN** the agent receives it as typed

#### Scenario: Refused request
- **WHEN** a paired phone asks to start a new session, rename, close or remove one, edit a group, close a shell or stop the host
- **THEN** the host answers with a refusal, nothing changes, and the phone stays connected

#### Scenario: Sending an image
- **WHEN** a paired phone uploads a screenshot to one of the host's sessions
- **THEN** the host saves it and answers with its path, and the phone stays connected
