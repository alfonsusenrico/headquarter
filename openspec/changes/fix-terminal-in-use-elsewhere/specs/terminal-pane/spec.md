## ADDED Requirements

### Requirement: In use on another device
While a paired phone is in use for the session shown in the terminal pane or the shell panel, the system SHALL show a bar at the top of that terminal reading "In use on <phone> · Click to use here" and SHALL keep the terminal visible below it. Clicking the bar or the terminal, or typing into it, SHALL make the desktop the client in use. While another client on the computer is in use, the bar SHALL read "In use in another terminal". The bar SHALL meet WCAG 2.2 AA contrast in every theme.

#### Scenario: The phone opens the session
- **WHEN** the desktop shows "cash flow tracker dev" and the owner opens it on the phone "Pixel 8"
- **THEN** the desktop's terminal shows "In use on Pixel 8 · Click to use here" above the agent's narrower drawing

#### Scenario: Click the bar
- **WHEN** the owner clicks the bar
- **THEN** the bar goes away and the agent redraws at the desktop's size

#### Scenario: One device
- **WHEN** only the desktop shows a session
- **THEN** no bar is shown
