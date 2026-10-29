## ADDED Requirements

### Requirement: A reply notifies once the session settles
The system SHALL post a "replied" notification only once the session has stayed unread, with no background work showing, for 10 seconds, and SHALL post none if in that time it works again, needs the owner, is read, or leaves the sidebar. Background work that ends while the session is unread SHALL start the 10 seconds then.

#### Scenario: A plain reply
- **WHEN** the Codex session "Release notes" finishes a turn while the owner works in another app and stays at its prompt
- **THEN** a notification reading "Codex replied" appears 10 seconds later

#### Scenario: Background work still runs
- **WHEN** a Claude Code session ends its turn while a background agent it started still runs
- **THEN** no notification appears, and "Claude replied" appears 10 seconds after the agent ends if the session stays at its prompt

#### Scenario: The agent carries on
- **WHEN** a session finishes a turn and starts working again 4 seconds later without the owner
- **THEN** no notification appears for that turn
