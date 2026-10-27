## ADDED Requirements

### Requirement: Quitting stops the host
Quitting the app, by its shortcut, its menu, or the window's close button, SHALL ask whether to stop the host and its sessions (Quit), quit the app only and leave them running (Keep Running), or stay (Cancel). Quit SHALL stop the host before the app ends, and the sessions that were running SHALL resume the next time the app opens.

#### Scenario: Quit with sessions running
- **WHEN** the owner presses ⌘Q with three sessions running and chooses Quit
- **THEN** the host and its agents stop, the app quits, and the three sessions resume when Headquarter opens again

#### Scenario: Keep running
- **WHEN** the owner closes the window and chooses Keep Running
- **THEN** the app quits and the host and its agents go on running

#### Scenario: Cancel
- **WHEN** the owner presses ⌘Q and chooses Cancel
- **THEN** the app stays open and nothing stops
