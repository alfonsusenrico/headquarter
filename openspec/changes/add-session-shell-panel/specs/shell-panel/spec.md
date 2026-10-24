# Spec Delta

## ADDED Requirements

### Requirement: Shell panel per session
The system SHALL open, on Cmd-T (Ctrl-Shift-T on Linux), a shell panel under the shown session's agent, running the owner's login shell in the session's folder, without changing the sidebar, and SHALL close the panel and end its shell on Cmd-T again.

#### Scenario: Opening
- **WHEN** the owner presses Cmd-T while a session is shown
- **THEN** a shell in the session's folder appears under the agent, takes the keyboard, and the sidebar is unchanged

#### Scenario: Closing with the shortcut
- **WHEN** the panel is open and the owner presses Cmd-T
- **THEN** the panel closes, its shell is ended, and the agent takes the keyboard

#### Scenario: Linux
- **WHEN** the owner presses Ctrl-T in a session on Linux
- **THEN** the agent receives the key and no panel opens; Ctrl-Shift-T opens it

### Requirement: Shells outlive switching
The system SHALL keep a session's shell running while the owner switches to other sessions or restarts the app, SHALL show it again when the session is shown, and SHALL end it when it exits or when its session is closed.

#### Scenario: Switching away
- **WHEN** a build runs in session A's shell and the owner opens session B, then A again
- **THEN** the build kept running and A's panel shows it

#### Scenario: Exit
- **WHEN** the owner types `exit` in the shell
- **THEN** the panel closes

#### Scenario: Session closed
- **WHEN** the owner closes a session whose shell is running
- **THEN** the shell ends with it, without another question

### Requirement: Shells are not agents
The system SHALL NOT track status, notify, import history or read usage for shells, and SHALL treat an agent started inside a shell as running in another terminal.

#### Scenario: Claude started in the shell
- **WHEN** the owner runs `claude` in a session's shell
- **THEN** it is listed as running in another terminal, and the session's own status is unaffected

### Requirement: Panel size
The system SHALL let the owner drag the divider between the agent and the shell, SHALL keep each part at least 6 rows tall, and SHALL remember the shell's share across restarts, one third by default.

#### Scenario: Resized and restarted
- **WHEN** the owner drags the divider to half and restarts the app
- **THEN** the next panel opens at half
