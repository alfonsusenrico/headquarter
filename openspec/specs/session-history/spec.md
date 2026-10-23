# session-history Specification

## Purpose
Shows the owner all of their agent conversations in one list, including past ones and ones running in other terminals, grouped by folder with how long ago each was active, without storing any conversation text.

## Requirements

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

### Requirement: One row per conversation
The system SHALL show a conversation that Headquarter hosts and that also appears in the agent's history as a single row.

#### Scenario: Hosted Claude session imported
- **WHEN** a Claude session started with Headquarter also appears in Claude's history
- **THEN** the list shows one row for it with its hosted status

### Requirement: Titles shown, never stored
The system SHALL display each conversation's title as the agent records it, SHALL tell connected clients when a conversation's title appears, changes, or goes away, and SHALL NOT write titles, prompts, or model output to its index, logs, or any file.

#### Scenario: Agent provides a title
- **WHEN** an agent has recorded a title for a conversation
- **THEN** the list shows that title for the row

#### Scenario: Title recorded after the client listed
- **WHEN** a client listed a conversation before the host read its title, and the conversation has no new activity
- **THEN** the client shows the title after the host's next history pass, without listing again

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
The system SHALL mark conversations whose agent is running in another terminal as external, with the process id and the best-known status, SHALL link an agy process to the conversation named on its command line or else to the conversation in its folder written most recently since the process started, unless other agy processes share that folder, and SHALL list running outside agents it cannot link to a conversation as separate external entries.

#### Scenario: Claude running elsewhere
- **WHEN** a Claude conversation in the list is open in another terminal
- **THEN** its row is marked external with that process's id and Claude's reported status

#### Scenario: agy waiting for the owner elsewhere
- **WHEN** agy runs in another terminal, its conversation was written since it started, and it is waiting for the owner's approval
- **THEN** that conversation's row is marked external and no separate entry is listed for the process

#### Scenario: Two agy processes in one folder
- **WHEN** two agy processes run in the same folder without a conversation named on their command lines
- **THEN** neither is linked by time, and each is listed as a separate external entry

#### Scenario: Outside agent without a known conversation
- **WHEN** an agy process is running in another terminal and no conversation can be linked to it
- **THEN** the list shows an external entry for it under its folder with status unknown

### Requirement: Conversations the owner started
The system SHALL treat as the owner's own only top-level conversations a person started: SHALL NOT import an agent's sub-agents as conversations, SHALL mark runs a program started (Claude Code's `claude -p` or Agent SDK, Codex's `codex exec`), Claude Code transcripts without a prompt or reply, and agy conversations never used as not offered, SHALL mark a past conversation its agent no longer keeps as not offered, and SHALL keep every such row in its index and in `hq ls --all`.

#### Scenario: Codex sub-agent
- **WHEN** Codex keeps a rollout whose source is a sub-agent
- **THEN** it is not imported as a conversation of its own

#### Scenario: Scripted run
- **WHEN** a Claude Code transcript's entrypoint is `sdk-cli`
- **THEN** its row is listed by `hq ls --all` with `offered` false

#### Scenario: Claude Code stub of a continued conversation
- **WHEN** a Claude Code transcript holds no prompt or reply, only a `continued-in` record naming the session the conversation went on in
- **THEN** its row is not offered, and the session it went on in is

#### Scenario: Conversation the agent deleted
- **WHEN** Claude Code deletes an old transcript whose conversation Headquarter had imported
- **THEN** after the next history pass its row is still listed by `hq ls --all`, with `offered` false, and clients are told of the change

### Requirement: Untitled past conversations
The system SHALL show a past conversation that its agent gave no title and the owner no name as "Untitled conversation" followed by its short id, the part of its generated name that follows the agent's prefix, and SHALL keep that generated name for the command line.

#### Scenario: Claude conversation without a title
- **WHEN** a past Claude Code conversation named `claude-e5a1fa64` has no title
- **THEN** Add sessions shows "Untitled conversation" with "e5a1fa64" after it in muted text, and `hq resume claude-e5a1fa64` still continues it

#### Scenario: A title arrives later
- **WHEN** the agent later writes a title for that conversation
- **THEN** the title is shown instead
