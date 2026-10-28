## ADDED Requirements

### Requirement: In use on another device
While another client is in use for the session shown in the terminal pane or the shell panel, the system SHALL show a notice at the top of that terminal and SHALL keep the terminal visible below it. The notice SHALL read "In use on <phone> · Click to use here" for a paired phone, and "In use in another terminal · Click to use here" for another client on the computer. It SHALL meet WCAG 2.2 AA contrast in every theme.

#### Scenario: The phone opens the session
- **WHEN** the desktop shows "cash flow tracker dev" and the owner opens it on the phone "Pixel 8"
- **THEN** the desktop's terminal shows "In use on Pixel 8 · Click to use here" above the agent's narrower drawing

#### Scenario: One device
- **WHEN** only the desktop shows a session
- **THEN** no notice is shown

### Requirement: Taking the session back on the desktop
Clicking the "In use" notice or the terminal, or typing into it, SHALL make the desktop the client in use, after which the notice goes away and the agent redraws at the desktop's size.

#### Scenario: Click the notice
- **WHEN** the owner clicks "In use on Pixel 8 · Click to use here"
- **THEN** the notice goes away and the agent redraws at the desktop's size

#### Scenario: Typing takes it back
- **WHEN** the notice is shown and the owner types into the terminal
- **THEN** the keys reach the agent, the notice goes away and the agent redraws at the desktop's size
