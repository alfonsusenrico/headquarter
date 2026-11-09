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
The app SHALL open a running session at the phone's width, show its terminal with scrollback, and send typed text, new lines, Enter and the key row's keys to it. Opening or typing SHALL make the phone the device in use, so the agent redraws at the phone's width. The terminal SHALL lay out a row's text again only when the row's content changes, not when the row only moves.

#### Scenario: Answering a prompt
- **WHEN** an agent shows a numbered permission prompt and the owner taps 1 in the key row
- **THEN** the agent receives the choice as if typed on the desktop

#### Scenario: Typing a message
- **WHEN** the owner types a message into the agent's box and taps ⏎ in the key row
- **THEN** the agent receives Enter after the typed text and sends the message

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
The app SHALL store only the phone's key (encrypted by the Android Keystore and excluded from backups), the paired computers' names, keys and relays, and its settings. It SHALL NOT store or log prompts, terminal output, session titles, pairing codes, keys or images it sends.

#### Scenario: After use
- **WHEN** the owner has used sessions on the phone and closes the app
- **THEN** the app's storage holds no terminal content, titles or codes, and its log only connection events

#### Scenario: After sending an image
- **WHEN** the owner has sent a photo, converted to JPEG, to a session
- **THEN** neither the photo nor the JPEG is in the app's storage or cache

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

### Requirement: Errors read plainly
The app SHALL show each error from the phone's core as the sentence the core gives, with no field names, and SHALL name a lost connection when the core reports one without a reason.

#### Scenario: Pairing fails
- **WHEN** pairing fails because the computer did not answer
- **THEN** the app shows "Couldn't reach the computer: the computer did not answer", without "reason="

#### Scenario: Not connected
- **WHEN** an action fails because the phone is not connected
- **THEN** the app shows "Not connected to the computer."

### Requirement: Scrolling a full-screen agent
Dragging the terminal on the phone SHALL scroll as the mouse wheel does on the desktop: as wheel reports at the dragged cell when the agent asked for mouse reports, as arrow keys on the alternate screen without them, and through the phone's own history otherwise.

#### Scenario: Claude Code's conversation
- **WHEN** the owner drags down on a Claude Code session's terminal on the phone
- **THEN** Claude Code shows earlier parts of the conversation, as the desktop's wheel does

#### Scenario: An agent that prints inline
- **WHEN** the owner drags down on a session whose agent keeps no alternate screen
- **THEN** the phone shows earlier output from its history, with the Live button to return

### Requirement: Replies with background work stay in their group
Home's top section SHALL list a reply not read yet only once the session shows no background work; until then the session SHALL stay in its group showing "background".

#### Scenario: Background work after the reply
- **WHEN** a Claude Code session ends its turn while its background shell still runs
- **THEN** the phone's home shows it in its group as "background", not at the top as "Claude replied"

#### Scenario: The work ends
- **WHEN** that shell ends and the reply is still unread
- **THEN** the session appears at the top as "Claude replied"

### Requirement: Session screen without a text box
The session and shell screens SHALL show the terminal flush to the screen's edges and every key without sideways scrolling while the keyboard is closed. They SHALL NOT show a text box of their own: the owner types into the agent's or shell's own input by tapping the terminal.

#### Scenario: Keys in reach
- **WHEN** the owner opens a session with the keyboard closed
- **THEN** 1, 2, 3, Esc, ⇧Tab, Tab, ←, ↑, ↓, →, Ctrl-C, the image key and ⏎ all show at once, ⏎ the largest, and no text box shows under them

#### Scenario: Typing folds the keys
- **WHEN** the owner taps the terminal and the keyboard opens
- **THEN** the keys become one row that scrolls sideways with ⏎ pinned at its end, and they return to the grid when the keyboard closes

#### Scenario: No Send button
- **WHEN** the owner types into the agent's box from the keyboard
- **THEN** no Send button shows; the key row's ⏎ is the only way to send

#### Scenario: Back to live output
- **WHEN** the owner has scrolled back in the terminal
- **THEN** a Live button shows, and tapping it returns to the newest output and hides the button

### Requirement: Typing into the agent's box
Tapping the terminal, away from a link, SHALL open the keyboard. Each edit the keyboard makes SHALL reach the session as it happens: new text as typed characters, and text the keyboard replaces or deletes as Backspaces before the replacement. The keyboard's autocorrect, swipe typing and voice typing SHALL keep working in an agent session. The key row's keys, and a tap that moves the cursor, SHALL start the next edit afresh, and Backspace SHALL reach the session even when nothing was typed since.

#### Scenario: Letters appear in the agent's box
- **WHEN** the owner taps the terminal of a Claude Code session and types "fix the test"
- **THEN** "fix the test" appears in Claude Code's own input box as it is typed, and nothing is sent

#### Scenario: A corrected word
- **WHEN** the owner types "teh " and the keyboard corrects it to "the "
- **THEN** the agent's box shows "the ", with the wrong word erased

#### Scenario: Erasing recalled text
- **WHEN** the owner taps ↑ in the key row, the agent recalls an earlier message, and the owner presses Backspace on the keyboard
- **THEN** the agent erases the last character of the recalled message

#### Scenario: Not connected
- **WHEN** the link to the computer drops while the owner types
- **THEN** "Reconnecting…" shows, the letters typed meanwhile are not sent later, and typing works again once connected

### Requirement: New line and send
In an agent session, the keyboard's Enter SHALL insert a new line in the agent's box (Ctrl+J), and only the key row's ⏎ SHALL send. In a shell, and in an agent whose new-line key is not known, the keyboard's Enter SHALL send Enter. A shell's input SHALL get no autocorrect and no automatic capitals.

#### Scenario: A message of two lines
- **WHEN** the owner types "first", presses the keyboard's Enter, types "second" and taps ⏎ in the key row
- **THEN** Claude Code receives one message of two lines

#### Scenario: A shell command
- **WHEN** the owner types `ls` in a session's shell and presses the keyboard's Enter
- **THEN** the shell runs `ls`

### Requirement: Links on the phone
Tapping a web address (`http` or `https`) in the terminal SHALL open it in the phone's browser and SHALL NOT open the keyboard or send anything to the session. A long press on one SHALL copy it and confirm the copy. The app SHALL find links an agent marks up (OSC 8) and plain-text addresses across wrapped rows, as the desktop does, and SHALL NOT open other kinds of links.

#### Scenario: Opening a link
- **WHEN** Claude Code prints a link it wraps over two rows and the owner taps the second row
- **THEN** the full address opens in the browser and the session receives nothing

#### Scenario: Copying a link
- **WHEN** the owner long-presses `https://example.com/docs.` in the shell
- **THEN** `https://example.com/docs` is copied, without the full stop, and a copy confirmation shows (the system's own on Android 13 and later, "Link copied" before)

#### Scenario: Not a web link
- **WHEN** an agent marks up a `file:` link and the owner taps it
- **THEN** nothing opens and the keyboard opens, as for a tap on text

#### Scenario: No browser
- **WHEN** no app on the phone can open web links and the owner taps a link
- **THEN** "No app can open this link" shows and nothing else happens

### Requirement: Sending an image
A session's screen SHALL have an image key that opens the system's photo picker, shown while the keyboard is open too. The app SHALL send the chosen image to the computer for that session and paste the path the computer answers into the agent's box, followed by a space. It SHALL show the sending's progress and say why when it fails, pasting nothing. A shell's screen SHALL have no image key.

#### Scenario: A screenshot to Claude Code
- **WHEN** the owner taps the image key in a Claude Code session and picks a screenshot
- **THEN** "Sending image…" shows with its progress, and then Claude Code's box shows `[Image #1]`

#### Scenario: With the keyboard open
- **WHEN** the owner is typing to Codex with the keyboard open and taps the image key beside ⏎
- **THEN** the photo picker opens, and the chosen image's path follows the typed text in Codex's box

#### Scenario: The connection drops
- **WHEN** the link to the computer drops while an image is sending
- **THEN** the banner says "The image didn't arrive" and nothing is pasted

### Requirement: Images the agents can read
Before sending, the app SHALL convert an image that is not PNG, JPEG, GIF or WebP, or that is larger than 10 MiB, to JPEG, scaled down until it is at most 10 MiB.

#### Scenario: A camera photo in HEIC
- **WHEN** the owner picks a 12 MB HEIC photo
- **THEN** the app sends it as a JPEG under 10 MiB and Claude Code attaches it

### Requirement: Moving the cursor
The key rows of sessions and shells SHALL have ← and →. A tap away from a link on the row the agent's cursor is on SHALL move the cursor to the tapped character with arrow presses, and no further than the end of the row's text. A tap on another row SHALL only open the keyboard.

#### Scenario: Correcting the middle of a message
- **WHEN** Claude Code's box holds "abcdef" with the cursor at its end, and the owner taps the "c" and types "X"
- **THEN** the box reads "abXcdef"

#### Scenario: Past the end
- **WHEN** the owner taps to the right of the text on the cursor's row
- **THEN** the cursor goes to the end of the text, not beyond

#### Scenario: Wide characters
- **WHEN** the box holds "日本x" and the owner taps the "本"
- **THEN** the cursor moves two characters left, one press per character

#### Scenario: Arrow keys
- **WHEN** the owner taps ← twice in the key row and types "Y"
- **THEN** "Y" is inserted two characters before where the cursor was
