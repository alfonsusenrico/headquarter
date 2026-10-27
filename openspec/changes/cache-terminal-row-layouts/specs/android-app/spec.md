## MODIFIED Requirements

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
