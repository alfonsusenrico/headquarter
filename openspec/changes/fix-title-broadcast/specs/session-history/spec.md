## MODIFIED Requirements

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
