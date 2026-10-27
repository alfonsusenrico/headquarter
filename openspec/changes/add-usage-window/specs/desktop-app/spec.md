## ADDED Requirements

### Requirement: Usage of sessions
The app SHALL show the CPU and memory each session in the sidebar uses, including every process the session's agent started and its shell panel, and the CPU and memory Headquarter itself uses (the app and the host together). Sessions that are not running SHALL read "not running" with no figures. The app SHALL measure only while these figures are shown.

#### Scenario: A stopped session
- **WHEN** a session in the sidebar is not running
- **THEN** its row reads "not running" and shows no CPU or memory

#### Scenario: Nothing open
- **WHEN** neither the popover nor the window is open
- **THEN** the app does not ask the host for these figures

### Requirement: Opening Usage
⌘U on macOS (Ctrl+Shift+U on Linux) SHALL open a popover with Headquarter's figures, the five running sessions using the most CPU, and a way to all of them. Show all, and Window ▸ Usage on macOS, SHALL open the full list.

#### Scenario: Quick look
- **WHEN** the owner presses ⌘U
- **THEN** a popover shows Headquarter's CPU and memory and the five running sessions using the most CPU

#### Scenario: Every session
- **WHEN** the owner chooses Show all in the popover, or Window ▸ Usage
- **THEN** a window lists every sidebar session with its CPU and memory, sortable by either, and a session that started other processes says how many
