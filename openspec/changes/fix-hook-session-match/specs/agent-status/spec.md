## ADDED Requirements

### Requirement: Status from the session's own conversation
The system SHALL change a Claude Code session's status, needs-you reason, and header details only from hook and status-line reports of that session's own conversation, and SHALL ignore reports from another conversation that carry the same session's launch environment. It SHALL follow the session to the conversation its own agent starts, including a resume that gives the conversation a new id and a `/clear` or `/compact` in place.

#### Scenario: A background Claude Code session waits
- **WHEN** a Claude Code process started by Claude Code's own daemon, with the session's launch environment, reports that it is waiting for input while the session works
- **THEN** the session stays working and its details are unchanged

#### Scenario: The owner clears the conversation
- **WHEN** the owner runs `/clear` in a session and Claude Code starts a new conversation in the same process
- **THEN** the session's status follows the new conversation's reports

#### Scenario: A resume with a new conversation id
- **WHEN** the host resumes a session and Claude Code continues it under a new conversation id
- **THEN** the session's status follows the reports of that new conversation
