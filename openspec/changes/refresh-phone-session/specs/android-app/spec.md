## ADDED Requirements

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
