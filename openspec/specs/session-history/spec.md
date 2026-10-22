# session-history Specification

## Purpose
Shows the owner all of their agent conversations in one list, including past ones and ones running in other terminals, grouped by folder with how long ago each was active, without storing any conversation text.

## Requirements

### Requirement: Import past conversations
The system SHALL list every past Claude Code, Codex, and agy conversation found in the agents' own history, each with its agent, working folder, and last activity time, without modifying any agent file.

#### Scenario: Conversation started before Headquarter
- **WHEN** the owner has a Claude conversation from last week that was never opened through Headquarter
- **THEN** the list shows it under its folder as inactive with its age since last activity

#### Scenario: Agent history missing
- **WHEN** an agent is not installed or has no history
- **THEN** the other agents' conversations are still listed and nothing fails

#### Scenario: Corrupt record
- **WHEN** one history file or record cannot be read
- **THEN** it is skipped and every other conversation is still listed

### Requirement: One row per conversation
The system SHALL show a conversation that Headquarter hosts and that also appears in the agent's history as a single row.

#### Scenario: Hosted Claude session imported
- **WHEN** a Claude session started with Headquarter also appears in Claude's history
- **THEN** the list shows one row for it with its hosted status

### Requirement: Titles shown, never stored
The system SHALL display each conversation's title as the agent records it, and SHALL NOT write titles, prompts, or model output to its index, logs, or any file.

#### Scenario: Agent provides a title
- **WHEN** an agent has recorded a title for a conversation
- **THEN** the list shows that title for the row

#### Scenario: Index contents after import
- **WHEN** the index is inspected after importing conversations with titles
- **THEN** it contains no title text, prompt text, or model output

#### Scenario: Title removed by the agent
- **WHEN** the agent no longer records a title for a conversation
- **THEN** the row shows no title after the next refresh

### Requirement: Fresh within a minute
The system SHALL reflect conversations started or continued outside Headquarter within one minute, without a restart.

#### Scenario: New conversation in another terminal
- **WHEN** the owner starts a new agent conversation in another terminal and uses it
- **THEN** within one minute the list includes it with a current age

### Requirement: Outside sessions in the list
The system SHALL mark conversations whose agent is running in another terminal as external, with the process id and the best-known status, and SHALL list running outside agents it cannot link to a conversation as separate external entries.

#### Scenario: Claude running elsewhere
- **WHEN** a Claude conversation in the list is open in another terminal
- **THEN** its row is marked external with that process's id and Claude's reported status

#### Scenario: Outside agent without a known conversation
- **WHEN** an agy process is running in another terminal and no conversation can be linked to it
- **THEN** the list shows an external entry for it under its folder with status unknown
