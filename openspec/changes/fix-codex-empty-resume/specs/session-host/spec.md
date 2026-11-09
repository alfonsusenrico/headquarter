## MODIFIED Requirements

### Requirement: Resume
The system SHALL reopen an inactive session's conversation inside the host using the agent's own resume mechanism. When the conversation id is unknown, it SHALL open Claude Code's own conversation picker in the session's folder, and start a new conversation there for Codex and agy: their sessions without an id are ones where nothing was sent. A Claude Code conversation that was never saved (no message sent) SHALL start afresh under the same id.

#### Scenario: Resume a known Claude conversation
- **WHEN** the owner resumes an inactive Claude session whose conversation id is recorded
- **THEN** a hosted session starts in the same folder continuing that conversation

#### Scenario: Conversation id unknown
- **WHEN** the owner resumes an inactive Claude session without a recorded conversation id
- **THEN** the agent starts in that folder showing its own conversation picker

#### Scenario: Agent without a picker
- **WHEN** the owner resumes an inactive agy session without a recorded conversation id
- **THEN** agy starts a new conversation in that folder

#### Scenario: Never used
- **WHEN** the owner resumes, or the host starts, a Claude session in which no message was ever sent
- **THEN** Claude Code starts a new conversation under the session's id, and no failure is reported

#### Scenario: A Codex session never used
- **WHEN** the host starts again with a Codex session in which nothing was ever sent
- **THEN** Codex starts a new conversation in its folder, and no picker of other conversations shows
