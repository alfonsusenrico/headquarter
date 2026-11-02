## ADDED Requirements

### Requirement: Claude usage with only weekly windows
The system SHALL read Claude's usage from a `/usage` screen or status line that reports only weekly windows, showing the week and its per-model windows, and SHALL show a five-hour window only while Claude Code reports one.

#### Scenario: Claude Code 2.1.316
- **WHEN** Claude Code's usage screen shows "Current week (all models) 6% used" and "Current week (Fable) 0% used" and no five-hour block
- **THEN** the details show the week at 6% and Fable's week at 0% with their resets, no five-hour row, and no update problem

#### Scenario: A five-hour window reported again
- **WHEN** a later reading includes a five-hour window
- **THEN** the details show it with its share and reset, and the ring gains its inner ring
