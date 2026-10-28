## ADDED Requirements

### Requirement: Update window keys
While the update window is open it SHALL hold the keyboard, so nothing typed reaches the session behind it. Escape SHALL close it whenever it offers OK, Later or Close, and closing it SHALL return the keyboard to what had it before.

#### Scenario: The latest version
- **WHEN** the owner chooses Check for Updates…, is told Headquarter is the latest version, and presses Escape
- **THEN** the notice closes and the session's terminal has the keyboard again

#### Scenario: Typing with the window open
- **WHEN** the update window is open over a session and the owner types
- **THEN** nothing reaches the session's agent

#### Scenario: While installing
- **WHEN** an update is installing and the owner presses Escape
- **THEN** the window stays open
