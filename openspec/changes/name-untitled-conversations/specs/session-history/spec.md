# Spec Delta

## ADDED Requirements

### Requirement: Untitled past conversations
The system SHALL show a past conversation that its agent gave no title and the owner no name as "Untitled conversation" followed by its short id, the part of its generated name that follows the agent's prefix, and SHALL keep that generated name for the command line.

#### Scenario: Claude conversation without a title
- **WHEN** a past Claude Code conversation named `claude-e5a1fa64` has no title
- **THEN** Add sessions shows "Untitled conversation" with "e5a1fa64" after it in muted text, and `hq resume claude-e5a1fa64` still continues it

#### Scenario: A title arrives later
- **WHEN** the agent later writes a title for that conversation
- **THEN** the title is shown instead
