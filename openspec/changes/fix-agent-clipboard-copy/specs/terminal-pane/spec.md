# Spec Delta

## MODIFIED Requirements

### Requirement: Clipboard
The system SHALL copy selected text to the system clipboard, SHALL put text the agent asks the terminal to copy (OSC 52) on the system clipboard without ever letting the agent read the clipboard, and SHALL paste clipboard text to the agent, wrapped for bracketed paste when the agent enabled it.

#### Scenario: Paste into an agent prompt
- **WHEN** the owner pastes several lines while the agent has bracketed paste enabled
- **THEN** the agent receives the text between bracketed-paste markers

#### Scenario: Copied by the agent
- **WHEN** the owner selects text with the mouse inside Claude Code, which copies it with OSC 52
- **THEN** the text is on the system clipboard, as in Alacritty

#### Scenario: Agent asks to read the clipboard
- **WHEN** an agent sends an OSC 52 request to read the clipboard
- **THEN** nothing is sent back and the clipboard is unchanged

### Requirement: Scrollback and mouse
The system SHALL keep scrollback for the normal screen, SHALL select text when the owner drags with the left button and copy it to the clipboard on release, for every agent, SHALL send clicks without a drag and wheel events to an agent that enabled mouse reporting, and SHALL otherwise use clicks to clear the selection and the wheel to scroll.

#### Scenario: Scrolling back
- **WHEN** the owner scrolls up on the normal screen of an agent that did not enable mouse reporting
- **THEN** earlier output is shown without sending anything to the agent

#### Scenario: Dragging over Claude Code
- **WHEN** the owner drags across text in a Claude Code session that enabled mouse reporting, then releases
- **THEN** the text is selected, the clipboard holds it, and nothing of the drag is sent to Claude Code

#### Scenario: Clicking in an agent that uses the mouse
- **WHEN** the owner clicks without moving in a session whose agent enabled mouse reporting
- **THEN** the agent receives the press and the release at that cell

#### Scenario: Word and line
- **WHEN** the owner double-clicks a word, or triple-clicks a line
- **THEN** the word or line is selected and copied
