## MODIFIED Requirements

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

## ADDED Requirements

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
