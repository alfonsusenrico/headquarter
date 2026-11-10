## MODIFIED Requirements

### Requirement: The app on Windows
On Windows the app SHALL be a client of remote hosts only: it SHALL open at Connect to a Host until a host is paired, keep its data in `%APPDATA%\Headquarter`, and never look for or start a host on the PC. It SHALL open no console window for any program it starts, SHALL carry the Headquarter icon in its executable, and SHALL draw minimize, maximize or restore, and close buttons at its top right that Windows treats as the window's own.

#### Scenario: First launch on Windows
- **WHEN** the owner opens the app on Windows for the first time
- **THEN** setup asks only how the app should look (no agents step), then Connect to a Host opens, and after a valid code the app shows that host's sessions

#### Scenario: No local host
- **WHEN** the owner opens the Hosts menu on Windows
- **THEN** it lists only paired hosts and Connect to a Host

#### Scenario: No console windows
- **WHEN** the owner opens the app on Windows and it fetches the agents' marks and checks for an update
- **THEN** no console or command window appears

#### Scenario: The icon
- **WHEN** the owner looks at `headquarter-app.exe` in Explorer, the taskbar or Alt+Tab
- **THEN** each shows the Headquarter icon

#### Scenario: Window buttons
- **WHEN** the owner points at maximize, then clicks close
- **THEN** Windows shows its snap layouts on hover, and close closes the window as the app's close does (the quit question where it applies)

#### Scenario: Moving the window
- **WHEN** the owner drags the empty part of the pane header, then double-clicks it
- **THEN** the window moves, then maximizes
