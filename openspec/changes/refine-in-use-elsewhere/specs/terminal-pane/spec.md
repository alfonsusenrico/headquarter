## MODIFIED Requirements

### Requirement: In use on another device
While another client is in use for the session shown in the terminal pane or the shell panel, the system SHALL show, in place of that terminal, a page laid out as for an inactive session: the agent's mark, "Open on <phone>" for a paired phone or "Open in another terminal" for another client on the computer, and a "Use here" button. Keys pressed there SHALL NOT reach the agent. The page SHALL meet WCAG 2.2 AA contrast in every theme.

#### Scenario: The phone opens the session
- **WHEN** the desktop shows "cash flow tracker dev" and the owner opens it on the phone "Pixel 8"
- **THEN** the desktop's pane shows "Open on Pixel 8" and "Use here", and no terminal

#### Scenario: Keys on the page
- **WHEN** the page is shown and the owner types letters
- **THEN** nothing reaches the agent and the phone keeps the session

#### Scenario: One device
- **WHEN** only the desktop shows a session
- **THEN** the terminal is shown

## REMOVED Requirements

### Requirement: Taking the session back on the desktop
**Reason**: The terminal is no longer shown while another device has the session, so clicking or typing into it cannot take it back (owner, 2026-10-28).
**Migration**: See "Taking a session back from another device".

## ADDED Requirements

### Requirement: Taking a session back from another device
Pressing "Use here" or Enter on the in-use page, or opening the session from the sidebar, SHALL make the desktop the client in use, after which the terminal is shown and the agent redraws at the desktop's size. The desktop's window becoming active SHALL NOT take the session back.

#### Scenario: Use here
- **WHEN** the owner presses "Use here" on "Open on Pixel 8"
- **THEN** the terminal is shown and the agent redraws at the desktop's size

#### Scenario: Enter
- **WHEN** the page is shown and the owner presses Enter
- **THEN** the desktop takes the session back, and the Enter does not reach the agent

#### Scenario: Back at the Mac
- **WHEN** the phone has the session and the owner switches to the desktop app's window
- **THEN** the page stays and the phone keeps the session
