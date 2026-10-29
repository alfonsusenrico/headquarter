# android-app Specification

## Purpose
The Android app lets the owner follow and answer the agent sessions on paired computers from a phone: it pairs by QR code, mirrors the desktop's sidebar, opens sessions and shells at the phone's width, connects only while on screen, and keeps no session content.

## Requirements

### Requirement: Pairing with a computer
The app SHALL pair with a computer from the QR code its Remote Control window shows, read by the camera or typed in, and SHALL keep each paired computer so it reconnects without pairing again.

#### Scenario: First launch
- **WHEN** the owner opens the app for the first time and scans the QR code shown under Pair a Phone
- **THEN** the computer's window says the phone is paired, and the app shows that computer's home

#### Scenario: Typed code
- **WHEN** the camera is refused or cannot read the screen, and the owner enters the code shown under the QR code
- **THEN** the phone pairs as with a scan

#### Scenario: Expired code
- **WHEN** the owner scans a code that expired or was already used
- **THEN** the app says the code no longer works and to show a new one on the computer, and nothing is saved

### Requirement: Several computers
The app SHALL keep every computer the phone paired with, show one at a time, and switch between them from the computer's name at the top of home. Only the computer on screen SHALL be connected.

#### Scenario: Switching
- **WHEN** the owner has paired a MacBook and a Studio, is on the MacBook's home, and picks Studio in the switcher
- **THEN** the app disconnects from the MacBook and shows Studio's sessions

#### Scenario: Pairing another computer
- **WHEN** the owner picks Pair another computer and scans a second computer's code
- **THEN** both computers are listed, and the new one is shown

### Requirement: Home mirrors the desktop
The app SHALL show the selected computer's sessions as its desktop sidebar does, with sessions that need the owner and replies not read yet first, then the owner's groups in the owner's order with their colours, folding and unfolding, then sessions in no group, each with its status and age, and the provider usage the desktop shows. Changes made on either side SHALL appear on both.

#### Scenario: A session needs the owner
- **WHEN** an agent on the computer asks for permission while the app shows home
- **THEN** that session appears at the top as a needs-you row within a second or two

#### Scenario: Seen on the desktop
- **WHEN** the owner opens a session with an unread reply on the desktop
- **THEN** its unread mark clears on the phone

### Requirement: Session on the phone
The app SHALL open a running session at the phone's width, show its terminal with scrollback, and send typed text, Enter and the key row's keys to it. Opening or typing SHALL make the phone the device in use, so the agent redraws at the phone's width. The terminal SHALL lay out a row's text again only when the row's content changes, not when the row only moves.

#### Scenario: Answering a prompt
- **WHEN** an agent shows a numbered permission prompt and the owner taps 1 in the key row
- **THEN** the agent receives the choice as if typed on the desktop

#### Scenario: Typing a message
- **WHEN** the owner types a message in the text box and taps Send
- **THEN** the agent receives the text and then Enter, as a typed message rather than a paste

#### Scenario: Text size
- **WHEN** the owner pinches the terminal larger
- **THEN** the text grows, fewer columns fit, and the agent redraws at the new width

#### Scenario: Fast output
- **WHEN** an agent prints lines quickly, so every row moves up each frame
- **THEN** the phone reuses the layouts of the rows that only moved and lays out only the new row

#### Scenario: Wide characters
- **WHEN** a row holds characters two cells wide, such as Chinese text
- **THEN** each part of the row still starts at its own column, as on the desktop

### Requirement: Shell, inactive sessions and adding sessions
The app SHALL open a running session's shell, offer Resume Session for an inactive session without starting anything until pressed, and list past conversations to add to the sidebar. It SHALL NOT start new sessions.

#### Scenario: Resume
- **WHEN** the owner opens an inactive session and taps Resume Session
- **THEN** the agent starts on the computer on that conversation and the session opens on the phone

#### Scenario: Add a past conversation
- **WHEN** the owner taps Add next to a past conversation
- **THEN** it appears in the sidebar on the phone and on the desktop

### Requirement: Connected only while on screen
The app SHALL connect while it is on screen, disconnect 30 seconds after it leaves the screen, and reconnect by itself when it returns or when the connection drops, keeping the last known list readable while it reconnects.

#### Scenario: Network lost
- **WHEN** the phone loses its network while showing home
- **THEN** the app shows Reconnecting with the last known list dimmed and its age, and resumes once the network returns

#### Scenario: Background
- **WHEN** the owner leaves the app for a minute
- **THEN** the computer no longer lists the phone as connected, and returning reconnects it

### Requirement: App lock
The app SHALL offer a lock, off by default, that the owner turns on in Settings. When it is on, the app SHALL lock itself when it opens and when it returns after more than a minute away, until the owner unlocks it with a fingerprint, face or the phone's PIN. While locked, the app SHALL NOT connect.

#### Scenario: A new install
- **WHEN** the owner opens the app for the first time
- **THEN** it opens unlocked, and Settings shows the app lock off

#### Scenario: Opening the app
- **WHEN** the app lock is on and the owner opens the app
- **THEN** only the lock screen shows until the phone confirms the owner

#### Scenario: No screen lock on the phone
- **WHEN** the phone has no screen lock
- **THEN** Settings says to set one before the app lock can be used, and the app opens unlocked

### Requirement: Unpairing from the phone
The app SHALL let the owner unpair a computer in Settings, which SHALL remove the phone from that computer's paired phones when it can be reached and SHALL forget the computer on the phone.

#### Scenario: Unpair
- **WHEN** the owner taps Unpair next to a connected computer
- **THEN** the computer's Remote Control window no longer lists the phone, and the app no longer shows that computer

#### Scenario: A stray tap
- **WHEN** the owner taps Unpair and then Cancel in the confirmation
- **THEN** nothing changes on the phone or the computer

#### Scenario: Removed on the computer
- **WHEN** the owner removed the phone on the computer and then opens the app
- **THEN** the app says the computer no longer knows this phone and offers to pair again

### Requirement: Versions and updates
The app SHALL say which side to update when the computer and the app speak different protocol versions, and SHALL, unless turned off, check at most once a day while open whether a newer GitHub release carries an Android APK, then offer it with a link to the release page.

#### Scenario: Older computer
- **WHEN** the computer runs a Headquarter older than the app needs
- **THEN** the app shows "Update Headquarter on <computer>" instead of an error

#### Scenario: Newer computer
- **WHEN** the computer runs a Headquarter newer than the app understands
- **THEN** the app shows "Update this app" with a button that opens the release page

#### Scenario: Updated meanwhile
- **WHEN** the app showed a version mismatch, the owner updates the other side and returns to the app
- **THEN** the app connects again without being restarted

#### Scenario: Newer APK
- **WHEN** the latest release is newer than the app and carries an APK
- **THEN** home shows that the new version is available, with Download opening the release page

### Requirement: The phone keeps no content
The app SHALL store only the phone's key (encrypted by the Android Keystore and excluded from backups), the paired computers' names, keys and relays, and its settings. It SHALL NOT store or log prompts, terminal output, session titles, pairing codes or keys.

#### Scenario: After use
- **WHEN** the owner has used sessions on the phone and closes the app
- **THEN** the app's storage holds no terminal content, titles or codes, and its log only connection events

### Requirement: Session screen layout
The session and shell screens SHALL show the terminal flush to the screen's edges, every key without sideways scrolling while the keyboard is closed, and a text box with a Send button inside it that appears only when the box holds text. Sending and the keys SHALL behave as before.

#### Scenario: Keys in reach
- **WHEN** the owner opens a session with the keyboard closed
- **THEN** 1, 2, 3, Esc, ⇧Tab, ↑, ↓, Tab, Ctrl-C and ⏎ all show at once, ⏎ the largest

#### Scenario: Typing folds the keys
- **WHEN** the owner taps the text box and the keyboard opens
- **THEN** the keys become one row that scrolls sideways with ⏎ pinned at its end, and they return to the grid when the keyboard closes

#### Scenario: Send appears with text
- **WHEN** the text box is empty
- **THEN** no Send button shows, and typing a character shows it inside the box

#### Scenario: Back to live output
- **WHEN** the owner has scrolled back in the terminal
- **THEN** a Live button shows, and tapping it returns to the newest output and hides the button

### Requirement: Home layout
Home SHALL show a large title that folds away while the list scrolls down and returns at the top, a full-width row for the computer with its connection state that opens the switcher and keeps Settings in reach, usage as one tile per agent, and a floating Add button that shrinks to an icon while the list scrolls down. What Home lists and how it behaves SHALL stay as before.

#### Scenario: The title folds away
- **WHEN** the owner scrolls the session list down
- **THEN** the "Headquarter" title row folds away while the computer row and Settings stay, and the title returns when the list is back at the top

#### Scenario: The computer row
- **WHEN** the phone is reconnecting to a computer named "Studio Mac"
- **THEN** the computer row shows the dot, "Studio Mac" and "reconnecting…", and tapping it opens the computers sheet

#### Scenario: Usage tiles
- **WHEN** the computer reports usage for Claude Code, Codex and agy
- **THEN** Home shows three tiles of equal width, each with the agent, the window, the percentage and a bar

#### Scenario: Add while scrolling
- **WHEN** the owner scrolls the list down while connected
- **THEN** the Add button shrinks to a round + icon, and it shows "Add" again when the owner scrolls up or reaches the top

### Requirement: One look for the phone's screens
Home, Pairing, Settings and Add sessions SHALL use vector icons for back, settings, add, search and chevrons, with a content description on every icon button, and keep their flows and wording.

#### Scenario: Pairing steps
- **WHEN** the owner opens Pair another computer
- **THEN** three numbered steps show above the camera, and "Enter the code instead" switches to the code field as before

#### Scenario: Icons are named
- **WHEN** TalkBack reads the back button on Settings
- **THEN** it says "Back"

### Requirement: Leaving a session another device took
The app SHALL take a running session or its shell when the owner opens it, without asking. When another client becomes the one in use for the open session, the app SHALL leave the session or shell screen for home and SHALL show for a few seconds "<session> is open on <computer>", or the other phone's name.

#### Scenario: The desktop takes the session back
- **WHEN** the phone shows "cash flow tracker dev" and the owner presses "Use here" on the desktop "MacBook"
- **THEN** the phone returns to home and shows "cash flow tracker dev is open on MacBook"

#### Scenario: Opening on the phone
- **WHEN** the desktop shows a session and the owner opens it on the phone
- **THEN** the phone shows the terminal at its width at once, with nothing to confirm

### Requirement: Home rows hold still while touched
While a finger is on the home list, or the list is still moving after a fling, the app SHALL keep each listed session in its place within its list and SHALL still update its status and age. A session new to a list SHALL take its place by the desktop's order. Once the list is at rest and untouched, the app SHALL apply the desktop's order.

#### Scenario: A session starts working during a scroll
- **WHEN** the owner scrolls the home list and a session lower in its group starts working
- **THEN** its row shows working in its current place, and moves to the top of its group once the list is at rest and untouched

#### Scenario: Not touched
- **WHEN** the home list is untouched and a session starts working
- **THEN** its row moves to the top of its group as on the desktop
