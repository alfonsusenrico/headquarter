## ADDED Requirements

### Requirement: Replies with background work stay in their group
Home's top section SHALL list a reply not read yet only once the session shows no background work; until then the session SHALL stay in its group showing "background".

#### Scenario: Background work after the reply
- **WHEN** a Claude Code session ends its turn while its background shell still runs
- **THEN** the phone's home shows it in its group as "background", not at the top as "Claude replied"

#### Scenario: The work ends
- **WHEN** that shell ends and the reply is still unread
- **THEN** the session appears at the top as "Claude replied"
