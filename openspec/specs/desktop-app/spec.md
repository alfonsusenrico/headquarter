# desktop-app Specification

## Purpose
Gives the owner one window that connects to the Headquarter host, lists sessions, starts new ones, and shows the chosen session, without ever ending sessions when the window closes.

## Requirements

### Requirement: Client of the host
The system SHALL connect to the Headquarter host as a client, starting it when needed, and SHALL leave every session running when the window or the app closes.

#### Scenario: App closed mid-turn
- **WHEN** the owner quits the app while an agent is working
- **THEN** the agent keeps working and `hq ls` shows it

### Requirement: Choose and start sessions
The system SHALL list running and recent sessions, keep the list current as sessions change, open the chosen session in the terminal pane, and start a new Claude Code, Codex, or agy session in a chosen folder.

#### Scenario: Switching sessions
- **WHEN** the owner selects another session in the list
- **THEN** the pane shows that session's agent and the previous one keeps running

#### Scenario: Start a session
- **WHEN** the owner starts a new session with an agent and a folder
- **THEN** it appears in the list and opens in the pane

#### Scenario: Host error shown
- **WHEN** starting a session fails, for example because the agent is not installed
- **THEN** the window shows the host's error message and nothing else changes

### Requirement: Themes
The system SHALL provide light and dark themes, with interface text meeting WCAG 2.2 AA contrast in both, and SHALL apply the theme the owner chose: System follows the operating-system appearance, Light and Dark are the built-in themes, and Alacritty uses the colours from the settings file. With no choice made, it SHALL use Alacritty when the settings file has colours and System otherwise.

#### Scenario: System switches to dark
- **WHEN** the theme is System and the operating system switches to dark appearance while the app is open
- **THEN** the window and terminal palette switch to the dark theme

#### Scenario: Dark chosen
- **WHEN** the owner has chosen Dark and the operating system switches to light appearance
- **THEN** the app stays dark

#### Scenario: Upgrade keeps the look
- **WHEN** the app starts for the first time after this change with Catppuccin colours imported in the settings file
- **THEN** it shows the Catppuccin colours, as before

#### Scenario: Alacritty colours removed
- **WHEN** Alacritty is the saved choice and the settings file no longer has colours
- **THEN** the app shows the System theme

### Requirement: Keyboard access
The system SHALL let the owner reach every function outside the terminal grid with the keyboard alone.

#### Scenario: Without a mouse
- **WHEN** the owner uses only the keyboard
- **THEN** they can move between the list and the pane, choose a session, and start a new one

### Requirement: Theme switcher
The system SHALL open a theme switcher on Cmd-G (Ctrl-Shift-G on Linux) that lists System, Light, Dark, the eight templates with their credits, Alacritty when the settings file has colours, and the owner's themes, applies each theme as the owner moves to it, keeps the chosen theme on Enter or a click, restores the previous theme on Escape or a click outside, remembers the choice across restarts, and offers Duplicate on fixed themes and Edit and Delete on the owner's themes.

#### Scenario: Preview and keep
- **WHEN** the owner presses Cmd-G, moves down to Light and presses Enter
- **THEN** the app turns light as Light is reached, the switcher closes, and the app is still light after a restart

#### Scenario: Preview and cancel
- **WHEN** the owner moves to Dark and presses Escape
- **THEN** the app returns to the theme it had before the switcher opened

#### Scenario: No Alacritty colours
- **WHEN** the settings file has no colours
- **THEN** the Alacritty entry is greyed, says the settings file has no colours, and cannot be chosen

#### Scenario: Ctrl-G on Linux
- **WHEN** the owner presses Ctrl-G in a session on Linux
- **THEN** the key reaches the agent and the switcher does not open

#### Scenario: Templates
- **WHEN** the owner opens the switcher
- **THEN** Catppuccin Mocha, Dracula, Nord, Gruvbox Dark, Tokyo Night, Catppuccin Latte, Solarized Light and Gruvbox Light are listed under "Templates", each with its credit

#### Scenario: Delete the theme in use
- **WHEN** the owner deletes their theme that is in use and confirms
- **THEN** its file is removed, the app shows System, and the theme is no longer listed

### Requirement: Window keys
The system SHALL quit on Cmd-Q, minimise the window on Cmd-M and hide the app on Cmd-H on macOS, and quit on Ctrl-Shift-Q and minimise on Ctrl-Shift-M on Linux; quitting SHALL leave hosted sessions running in the host.

#### Scenario: Quit with sessions running
- **WHEN** the owner presses Cmd-Q while a Claude Code session is working
- **THEN** the app quits, the session keeps working in the host, and it is shown again when the app reopens

#### Scenario: Minimise
- **WHEN** the owner presses Cmd-M
- **THEN** the window goes to the Dock and comes back when the app is chosen again

#### Scenario: Plain Ctrl-Q on Linux
- **WHEN** the owner presses Ctrl-Q in a session on Linux
- **THEN** the key reaches the agent and the app does not quit

### Requirement: Theme creator
The system SHALL let the owner create a theme from "+ Add theme", or from Duplicate or Edit, in a side panel that keeps the app in view, starting from any theme. The panel SHALL offer four seeds from which the other colours are made, every interface and terminal colour with its own opacity, and the window's background opacity and blur. Each change SHALL apply to the app at once; Save SHALL keep the theme and use it, and Cancel SHALL restore the previous theme.

#### Scenario: New theme from seeds
- **WHEN** the owner presses "+ Add theme", starts from Nord, sets the accent seed to orange and saves as "Nord orange"
- **THEN** the app uses Nord's colours with an orange accent, "Nord orange" is listed under "Yours", and it is still in use after a restart

#### Scenario: A pinned colour
- **WHEN** the owner sets the dividers colour by hand and then changes the background seed
- **THEN** the dividers keep the owner's colour while the other made colours follow the new background

#### Scenario: Colour opacity
- **WHEN** the owner sets the sidebar colour's opacity to 60 % with the window's background opacity at 90 %
- **THEN** the sidebar shows the content behind the window at 54 % opacity

#### Scenario: Cancel
- **WHEN** the owner changes several colours and presses Cancel
- **THEN** the app returns to the theme in use before and nothing is saved

### Requirement: Theme files
The system SHALL keep each of the owner's themes as two files in `themes/` next to the settings file: a colour file in Alacritty's format only, and a `.headquarter.toml` companion with the name, interface colours and opacities. It SHALL write, edit and delete both together, each file whole. It SHALL list a colour file without a companion as a theme named after the file, and SHALL list a file it cannot read as greyed with the reason, without changing it.

#### Scenario: Usable in Alacritty
- **WHEN** the owner saves a theme and imports its colour file in Alacritty
- **THEN** Alacritty applies its terminal colours, opacity and blur with no warning

#### Scenario: An Alacritty theme dropped in
- **WHEN** the owner copies `gruvbox_material.toml` from the Alacritty theme collection into `themes/`
- **THEN** the switcher lists "gruvbox_material" under "Yours", with interface colours made from its colours

#### Scenario: Broken file
- **WHEN** a file in `themes/` is not valid TOML
- **THEN** the switcher lists it greyed with the error and the file is left as it is

### Requirement: Redraw only what changed
The app SHALL redraw the sidebar only when something it shows changes, and SHALL redraw the shown terminal only when its contents change, so output in one does not repaint the other.

#### Scenario: Busy terminal
- **WHEN** the shown session's agent writes output and nothing in the sidebar changes
- **THEN** the app repaints the terminal and reuses the sidebar from the previous frame

#### Scenario: Sidebar change
- **WHEN** a sidebar row's status changes or its "working" word breathes and the shown terminal has no new output
- **THEN** the app repaints the sidebar and reuses the terminal from the previous frame

### Requirement: Idle window rests
The app SHALL stop asking the display for frames once its window has had nothing to draw for one second, and SHALL draw again at the next change.

#### Scenario: Nothing changes
- **WHEN** the window is visible and nothing on screen has changed for a second
- **THEN** the app wakes no more than about ten times a second

#### Scenario: Change after rest
- **WHEN** the owner types into the shown session, or its agent writes output, after the window has rested
- **THEN** the change is drawn at the next display refresh

### Requirement: Starting shows the agents checking in
The system SHALL show, until the host answers with its first session list, the agents' marks lighting up in turn above "Starting Headquarter…" in the main pane, and SHALL show neither the empty sidebar's hints nor the pane's hints until then.

#### Scenario: First seconds after opening
- **WHEN** the app opens and the host has not answered yet
- **THEN** the main pane shows the marks lighting up in turn and the sidebar shows no hint, until the sessions are listed

#### Scenario: Add sessions while starting
- **WHEN** the owner opens Add sessions before the first list has arrived
- **THEN** it shows the marks lighting up in turn above "Loading your sessions…" rather than "No sessions match."

### Requirement: Bring a background conversation into Headquarter
When a session's conversation runs in Claude Code's background, the app SHALL say so on the session pane and offer to bring it into Headquarter, which stops it there and resumes it in the app.

#### Scenario: Opening a session that runs in the background
- **WHEN** the owner opens a session whose conversation runs in Claude Code's background
- **THEN** the pane says so, and Bring into Headquarter resumes it in the app

### Requirement: One entry per conversation chain
Add sessions SHALL show a chain of conversations linked by Claude Code's continuations once, as its newest conversation.

#### Scenario: A conversation continued twice
- **WHEN** a conversation was continued under two later ids
- **THEN** Add sessions shows one entry, which opens the newest

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

### Requirement: Keyboard Shortcuts window
The app SHALL list every app shortcut in a Keyboard Shortcuts window, opened with ⌘/ on macOS (Ctrl+Shift+/ on Linux) or Headquarter ▸ Keyboard Shortcuts… on macOS, each with the keys for the system it runs on and a one-line explanation, taken from the same table the app binds its keys from. Typing SHALL filter the list at once.

#### Scenario: Opening the list
- **WHEN** the owner presses ⌘/ on macOS
- **THEN** a window lists the app's shortcuts by area, each with its keys and what it does

#### Scenario: Typing filters
- **WHEN** the window is open and the owner types "shell"
- **THEN** only the shortcuts whose name, explanation or key matches remain, and Esc shows them all again

#### Scenario: Linux keys
- **WHEN** the app runs on Linux and the window is open
- **THEN** each shortcut shows Ctrl Shift with its key
