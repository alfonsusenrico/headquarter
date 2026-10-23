# Spec Delta

## MODIFIED Requirements

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
