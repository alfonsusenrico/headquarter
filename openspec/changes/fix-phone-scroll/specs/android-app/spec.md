## ADDED Requirements

### Requirement: Scrolling a full-screen agent
Dragging the terminal on the phone SHALL scroll as the mouse wheel does on the desktop: as wheel reports at the dragged cell when the agent asked for mouse reports, as arrow keys on the alternate screen without them, and through the phone's own history otherwise.

#### Scenario: Claude Code's conversation
- **WHEN** the owner drags down on a Claude Code session's terminal on the phone
- **THEN** Claude Code shows earlier parts of the conversation, as the desktop's wheel does

#### Scenario: An agent that prints inline
- **WHEN** the owner drags down on a session whose agent keeps no alternate screen
- **THEN** the phone shows earlier output from its history, with the Live button to return
