# Spec Delta

## MODIFIED Requirements

### Requirement: Import past conversations
The system SHALL list every past Claude Code, Codex, and agy conversation found in the agents' own history, each with its agent, working folder, and last activity time, without modifying any agent file except when the owner removes a conversation from Add sessions.

#### Scenario: Conversation started before Headquarter
- **WHEN** the owner has a Claude conversation from last week that was never opened through Headquarter
- **THEN** the list shows it under its folder as inactive with its age since last activity

#### Scenario: Agent history missing
- **WHEN** an agent is not installed or has no history
- **THEN** the other agents' conversations are still listed and nothing fails

#### Scenario: Corrupt record
- **WHEN** one history file or record cannot be read
- **THEN** it is skipped and every other conversation is still listed
