## MODIFIED Requirements

### Requirement: Quitting stops the host
Quitting the app, by its shortcut, its menu, or the window's close button, SHALL ask whether to stop the host and its sessions (Quit), quit the app only and leave them running (Keep Running), or stay (Cancel). Quit SHALL stop the host before the app ends, and the sessions that were running SHALL resume the next time the app opens. The question SHALL show above any window the app has open, and Cancel SHALL return to that window as it was.

#### Scenario: Quit with sessions running
- **WHEN** the owner presses ⌘Q with three sessions running and chooses Quit
- **THEN** the host and its agents stop, the app quits, and the three sessions resume when Headquarter opens again

#### Scenario: Keep running
- **WHEN** the owner closes the window and chooses Keep Running
- **THEN** the app quits and the host and its agents go on running

#### Scenario: Cancel
- **WHEN** the owner presses ⌘Q and chooses Cancel
- **THEN** the app stays open and nothing stops

#### Scenario: Quit with a window open
- **WHEN** the Remote Control window is open and the owner presses ⌘Q, then Esc, then Esc again
- **THEN** the question shows above the window, the first Esc cancels it, and the second closes the Remote Control window
