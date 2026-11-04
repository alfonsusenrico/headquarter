## ADDED Requirements

### Requirement: Claude usage never goes back within a window
The system SHALL NOT lower a Claude window's share because of a hosted session's status-line report for the same window, SHALL show a new window's share once its period begins, and SHALL show a `/usage` read as Claude Code reports it.

#### Scenario: An idle session redraws its status line
- **WHEN** the week was saved at 59% and an idle hosted session's status line then reports the same week at 58%
- **THEN** the details keep showing 59% for the week

#### Scenario: The status line reports more
- **WHEN** the week was saved at 59% and a hosted session's status line reports the same week at 60%
- **THEN** the details show 60%

#### Scenario: A new week
- **WHEN** the week was saved at 97% and a status-line report gives a week that resets a week later at 2%
- **THEN** the details show 2% with the new reset

#### Scenario: Refresh reads a lower share
- **WHEN** the owner presses Refresh and `/usage` reports the week at 57% after the status line reported 59% for the same week
- **THEN** the details show 57%
