## MODIFIED Requirements

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

## ADDED Requirements

### Requirement: Session screen without a text box
The session and shell screens SHALL show the terminal flush to the screen's edges and every key without sideways scrolling while the keyboard is closed. They SHALL NOT show a text box of their own: the owner types into the agent's or shell's own input by tapping the terminal.

#### Scenario: Keys in reach
- **WHEN** the owner opens a session with the keyboard closed
- **THEN** 1, 2, 3, Esc, ⇧Tab, ↑, ↓, Tab, Ctrl-C and ⏎ all show at once, ⏎ the largest, and no text box shows under them

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
Tapping the terminal, away from a link, SHALL open the keyboard. Each edit the keyboard makes SHALL reach the session as it happens: new text as typed characters, and text the keyboard replaces or deletes as Backspaces before the replacement. The keyboard's autocorrect, swipe typing and voice typing SHALL keep working in an agent session. The key row's keys SHALL start the next edit afresh, and Backspace SHALL reach the session even when nothing was typed since.

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

## REMOVED Requirements

### Requirement: Session screen layout
**Reason**: The screen no longer has a text box with a Send button; "Session screen without a text box" replaces it with the same keys and Live button.
**Migration**: None: the owner types into the agent's own box by tapping the terminal.
