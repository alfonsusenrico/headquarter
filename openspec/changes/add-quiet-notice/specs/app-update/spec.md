## ADDED Requirements

### Requirement: Manual check results as notices
The result of Check for Updates… when no update is available, or when the check fails, SHALL be shown as a quiet notice instead of the update window.

#### Scenario: Already on the latest version
- **WHEN** the owner chooses Check for Updates… and the running version is the latest
- **THEN** a notice reads "Headquarter 0.1.21 is the latest version." and the working pane stays uncovered

#### Scenario: The check fails
- **WHEN** a manual check cannot reach GitHub
- **THEN** a notice says why and stays until dismissed

## MODIFIED Requirements

### Requirement: Update window keys
While the update window is open it SHALL hold the keyboard, so nothing typed reaches the session behind it. Escape SHALL close it whenever it offers OK, Later or Close, and closing it SHALL return the keyboard to what had it before.

#### Scenario: The latest version
- **WHEN** the owner chooses Check for Updates… and Headquarter is the latest version
- **THEN** a quiet notice says so, no window opens, and the session's terminal keeps the keyboard

#### Scenario: Typing with the window open
- **WHEN** the update window is open over a session and the owner types
- **THEN** nothing reaches the session's agent

#### Scenario: While installing
- **WHEN** an update is installing and the owner presses Escape
- **THEN** the window stays open
