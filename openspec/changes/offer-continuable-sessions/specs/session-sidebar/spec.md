# Spec Delta

## MODIFIED Requirements

### Requirement: Add sessions offers only continuable conversations
The system SHALL leave out of Add sessions, and out of its counts, every conversation the host does not offer and every conversation whose working folder no longer exists, SHALL keep a session whose folder is unknown, and SHALL NOT delete such conversations from any agent's history or from its index.

#### Scenario: Conversation from a removed temporary folder
- **WHEN** an agy conversation was run in a temporary folder that has since been removed
- **THEN** Add sessions does not list it, and `hq ls --all` still does

#### Scenario: Folder still there
- **WHEN** a conversation's folder exists
- **THEN** Add sessions lists it as before

#### Scenario: Not offered by the host
- **WHEN** the host reports a conversation as not offered, such as a `codex exec` run or one Claude Code deleted
- **THEN** Add sessions does not list it, for every agent alike
