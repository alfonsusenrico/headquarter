# desktop-app Specification

## Purpose
Gives the owner one window that connects to the Headquarter host, lists sessions, starts new ones, and shows the chosen session, without ever ending sessions when the window closes.

## Requirements

### Requirement: Client of the host
The system SHALL connect as a client to this computer's host, starting it when needed, and to every paired host on another computer at once, and SHALL leave every session running when the window or the app closes.

#### Scenario: App closed mid-turn
- **WHEN** the owner quits the app while an agent is working
- **THEN** the agent keeps working and `hq ls` shows it

#### Scenario: Remote host on screen
- **WHEN** the owner has paired a server
- **THEN** the sidebar shows the server's sessions together with this computer's, and their terminals and usage come from the server

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

### Requirement: Sessions already in a folder
The New session window SHALL show how many sessions each folder holds, sidebar sessions and past conversations together. Starting in a folder that holds sessions not in the sidebar SHALL first list only those, each with its agent, title and when it was last used, to continue one or start a new session, unless the owner chose not to be asked again about them. Starting in a folder whose sessions are all in the sidebar SHALL start at once.

#### Scenario: Browsing
- **WHEN** the owner browses to a folder holding one sidebar session and two past conversations
- **THEN** its row shows "3 sessions"

#### Scenario: All in the sidebar
- **WHEN** the owner starts a session in a folder whose only session is in the sidebar
- **THEN** the new session starts without a question

#### Scenario: One forgotten session
- **WHEN** the owner starts a session in a folder with two sessions, one in the sidebar and one not
- **THEN** a dialog lists only the one not in the sidebar, and Continue adds it to the sidebar and resumes it

#### Scenario: Don't ask again
- **WHEN** the owner ticks "Don't ask again for these" and starts a new session
- **THEN** later starts in that folder do not offer those sessions, and a session forgotten later is still offered

### Requirement: Usage total
The Usage popover and the Usage window SHALL end with Headquarter's own figures, all running sessions' figures together, and their sum, each with CPU and memory, counting every running session even where the popover lists only some.

#### Scenario: Popover total
- **WHEN** the owner opens the Usage popover with 12 sessions running
- **THEN** below the five busiest it shows Headquarter, all 12 sessions and the total, with CPU and memory for each

### Requirement: Quiet notices
The app SHALL show short messages in a notice at the bottom right of the window that slides in, leaves by itself after 5 seconds unless the pointer is on it, can be dismissed, never takes the keyboard, and appears without sliding when reduced motion is on. A notice reporting a failure SHALL stay until dismissed.

#### Scenario: A notice leaves by itself
- **WHEN** a notice appears and the owner keeps typing in the session
- **THEN** the keys reach the session, and the notice slides out after 5 seconds

#### Scenario: Dismissed
- **WHEN** the owner presses the notice's close button
- **THEN** it slides out at once

### Requirement: Connect to a Host
The app SHALL pair with a host on another computer from a pairing code entered in Connect to a Host, keep the host in its list, and say plainly when the code is wrong, expired or the host unreachable.

#### Scenario: Pairing from the code
- **WHEN** the owner pastes a valid code from the server's `hq remote pair`
- **THEN** the app pairs, adds the server to its host list, and shows its sessions

#### Scenario: Expired code
- **WHEN** the owner enters a code that has expired
- **THEN** the window says the code expired and asks for a new one, and nothing is added

### Requirement: Reconnecting to a remote host
While a host is unreachable, the app SHALL keep that host's sessions in the list greyed with "Reconnecting…", retry by itself, and reattach its terminals when it returns, leaving the other hosts' sessions as they are. When the app gives up on a paired host (the host removed this computer, or the two run releases that cannot talk), it SHALL say so in the sidebar with the host's name and what to do, until the host lists its sessions again or is forgotten. When the releases differ, it SHALL keep trying every 30 seconds without the owner acting.

#### Scenario: Network drop
- **WHEN** the network to the server drops for 20 seconds
- **THEN** the server's sessions show greyed with "Reconnecting…", this computer's sessions stay as they are, and when the network returns the server's sessions and an open terminal on the server come back without the owner acting

#### Scenario: Removed on the host
- **WHEN** the owner removes this computer on the host
- **THEN** the app stops retrying that host, its sessions leave the list, and the sidebar and the Hosts menu say the host no longer knows this computer and to pair it again with Connect to a Host

#### Scenario: The host runs an older release
- **WHEN** the server runs Headquarter 0.2.0-alpha.2 (protocol 15) and the app 0.4.0-alpha.1 (protocol 16)
- **THEN** the sidebar says "<server> needs an update: it runs Headquarter 0.2.0-alpha.2, this app 0.4.0-alpha.1." and the Hosts menu shows the server as "needs an update" with that reason

#### Scenario: The app is the older side
- **WHEN** the server speaks a higher protocol than the app
- **THEN** the sidebar says "Update this app: <server> runs Headquarter <its version>, this app <app version>."

#### Scenario: The host is updated while the app runs
- **WHEN** the server needed an update and the owner updates it without touching the app
- **THEN** within about 30 seconds the server's sessions return to the list and the sidebar line goes away

#### Scenario: Opening with the host away
- **WHEN** the app opens and a paired host cannot be reached yet
- **THEN** this computer's sessions show at once, and the Hosts menu says the host is connecting

### Requirement: A remote host is never stopped from the app
Quitting or updating the app SHALL act on this computer only: it SHALL never stop, or ask about stopping, a host on another computer.

#### Scenario: Quit with the server on screen
- **WHEN** the owner quits the app while the server's sessions are in the list
- **THEN** the app asks only about this computer's running sessions, and the server's sessions keep running

### Requirement: Files stay on their computer
With a remote host's session on screen, dropping images onto its terminal, or pasting an image copied to the clipboard, SHALL send each image to that host and paste the path it answers, as a dropped file's path is pasted on this computer. Dropping any other file SHALL show a notice that only images can be sent to another computer and type nothing. File links SHALL open only when the host is this computer.

#### Scenario: Dropping a file onto a remote session
- **WHEN** the owner drops a PDF from the PC onto the server's session
- **THEN** a notice says only images can be sent to another computer, and nothing is typed into the session

#### Scenario: Dropping a screenshot onto a remote session
- **WHEN** the owner drops a PNG from the PC onto a Claude Code session on the server
- **THEN** "Sending image…" shows, the image is saved on the server, and Claude Code's box shows `[Image #1]`

#### Scenario: Pasting a screenshot into a remote session
- **WHEN** the owner takes a screenshot to the clipboard on Windows and presses the terminal's Paste key in a Codex session on the server
- **THEN** the image is sent as a PNG and Codex's box shows `[Image #1]`

#### Scenario: Pasting text is unchanged
- **WHEN** the clipboard holds text and the owner pastes into a remote session
- **THEN** the text is pasted as before and nothing is uploaded

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
- **WHEN** the owner drags the strip above the sidebar's Hosts menu, then double-clicks it
- **THEN** the window moves, then maximizes

### Requirement: Updates on Windows
On Windows a newer release SHALL show the update notice with a link to its release page instead of installing it.

#### Scenario: Newer release
- **WHEN** a newer release exists and the app on Windows checks for updates
- **THEN** the notice names the version and opens its release page when chosen

### Requirement: Finding this computer's host
The app SHALL use this computer's host only when the installer recorded one or `~/.local/bin/hq` exists, starting it when it is not running, and SHALL otherwise open at Connect to a Host and never start a host.

#### Scenario: App-only install
- **WHEN** the owner opens the app on a laptop with an app-only install
- **THEN** the app shows Connect to a Host, and the host switcher has no entry for this computer

#### Scenario: Full install
- **WHEN** the owner opens the app on a Mac with a full install and the host is not running
- **THEN** the app starts the host from `~/.local/bin/hq` and shows its sessions

### Requirement: Updates follow the installed parts
The app's updater SHALL install the same parts the installer recorded: the host and the app for a full install, the app only for an app-only install.

#### Scenario: Updating an app-only laptop
- **WHEN** the owner installs an update on an app-only laptop
- **THEN** only the app is updated and restarted, and no host is installed

### Requirement: Every host in one list
The sidebar SHALL list the sessions of this computer's host and of every paired host together. A session on another computer SHALL show that computer's name; on a computer without its own host, every session SHALL show its host's name. Every action on a session SHALL go to its host. When a row is too narrow for the title and the whole name, the name SHALL keep at least its first four characters before an ellipsis, and the title SHALL give way first.

#### Scenario: The Mac and the server
- **WHEN** the owner on the Mac has a server paired with a health agent running there
- **THEN** the sidebar lists the Mac's sessions without a host name and the health agent with "server"

#### Scenario: On Windows
- **WHEN** the owner on a Windows PC has one server paired
- **THEN** every session shows "server"

#### Scenario: Renaming a server session
- **WHEN** the owner renames the server's health agent from the Mac's sidebar
- **THEN** the server renames it, and the phone paired with the server shows the new name

#### Scenario: A long title on a selected row
- **WHEN** the selected row of "Server Management Agent Setup" on "msigs63vr" shows its close button
- **THEN** the tag reads at least "msig…", centred in its box, and the title is shortened instead

### Requirement: Groups across hosts
The app SHALL show groups of the same name on different hosts as one group. Moving a session into a group SHALL use that host's group of the name, making it there when missing. Renaming, recolouring, collapsing, moving and deleting a group SHALL apply on every host that has it.

#### Scenario: A server session joins the Mac's group
- **WHEN** the Mac has a group "ops" and the owner drags the server's health agent into it
- **THEN** the server gets a group "ops" holding the agent, and the sidebar shows one "ops" with the Mac's sessions and the agent

#### Scenario: Renaming a shared group
- **WHEN** "ops" exists on the Mac and on the server and the owner renames it to "infra"
- **THEN** both hosts rename their "ops" to "infra"

### Requirement: Hosts menu
The sidebar header SHALL open a Hosts menu listing this computer when it has a host and each paired host with its state (connected, connecting, reconnecting, needs an update, pair again, not reachable), Connect to a Host, and Forget for each paired host. A host the app gave up on SHALL show why under its row. While any paired host is not connected, the menu SHALL offer Reconnect all, which tries every such host again at once. Forget SHALL ask in place before forgetting: the row SHALL read "Forget <host>? You'll need a new code from it to connect again." with Cancel and Forget buttons of equal width, Forget in the danger colour, and Escape or a click elsewhere SHALL cancel.

#### Scenario: A misclick on Forget
- **WHEN** the owner clicks Forget on the server's row by mistake and presses Escape
- **THEN** the server stays paired and its sessions stay in the list

#### Scenario: Forgetting a host
- **WHEN** the owner clicks Forget, then Forget again in the confirmation
- **THEN** the server leaves the Hosts menu and its sessions leave the list

#### Scenario: Reconnect all
- **WHEN** the server shows "reconnecting" or "needs an update" and the owner chooses Reconnect all
- **THEN** the app tries the server again at once, and its sessions return as soon as it answers on the same release

#### Scenario: Every host connected
- **WHEN** every paired host is connected
- **THEN** the Hosts menu does not show Reconnect all

#### Scenario: Forgetting a host that needs an update
- **WHEN** the server needs an update and the owner forgets it
- **THEN** its line leaves the sidebar and the app stops trying it

### Requirement: Usage follows the selected session
The usage footer SHALL show the usage of the selected session's host, naming the host when it is not this computer, and this computer's usage when nothing is selected. The Usage window (CPU and memory) SHALL show the same host's sessions and name it when it is not this computer.

#### Scenario: A server session selected
- **WHEN** the owner selects the server's health agent
- **THEN** the footer shows the server's Claude and Codex usage, named "server"

### Requirement: New sessions on a chosen host
When more than one host is connected, New Session SHALL ask which host starts the session, the selected session's host first, and its folder browser SHALL show that host's folders.

#### Scenario: Starting on the server
- **WHEN** the owner opens New Session, chooses the server and a folder there
- **THEN** the session starts on the server and appears in the list with "server"
