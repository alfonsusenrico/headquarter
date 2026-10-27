## ADDED Requirements

### Requirement: Bring a background conversation into Headquarter
When a session's conversation runs in Claude Code's background, the app SHALL say so on the session pane and offer to bring it into Headquarter, which stops it there and resumes it in the app.

#### Scenario: Opening a session that runs in the background
- **WHEN** the owner opens a session whose conversation runs in Claude Code's background
- **THEN** the pane says so, and Bring into Headquarter resumes it in the app

### Requirement: One entry per conversation chain
Add sessions SHALL show a chain of conversations linked by Claude Code's continuations once, as its newest conversation.

#### Scenario: A conversation continued twice
- **WHEN** a conversation was continued under two later ids
- **THEN** Add sessions shows one entry, which opens the newest
