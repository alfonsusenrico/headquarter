## ADDED Requirements

### Requirement: Claude sessions follow their conversation
The host SHALL keep each Claude session on the newest conversation Claude Code continues it in, recording a continuation as soon as it appears, so a resume after any restart continues where the owner left off.

#### Scenario: Continued under a new id
- **WHEN** Claude Code carries a hosted session's conversation on under a new id and the host restarts
- **THEN** the session resumes the new id, not the old one

### Requirement: Conversations running in Claude Code's background
The host SHALL NOT start an agent to resume a Claude conversation that Claude Code reports running in its background, and SHALL answer that the conversation is running there.

#### Scenario: Resume while running in the background
- **WHEN** the owner resumes a session whose conversation Claude Code runs in its background
- **THEN** no agent starts and the answer says it runs in Claude Code's background, with its process id

### Requirement: Merges keep the owner's rows
The host SHALL NOT delete a session that is in the sidebar or named by the owner when it merges two records of one conversation, and SHALL log every change to sidebar membership with the session ids.

#### Scenario: A named sidebar row and an imported copy
- **WHEN** a named sidebar session and an imported history row come to hold the same conversation
- **THEN** the named sidebar session stays, and the imported row is removed
