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

#### Scenario: Copy confirmed
- **WHEN** the owner releases a drag over three lines
- **THEN** a note says "Copied 3 lines" at the bottom right of the pane for about 1.5 seconds

#### Scenario: Tidy copy
- **WHEN** the owner copies an indented reply whose lines end in spaces, with blank lines above and below
- **THEN** the clipboard holds the lines without trailing spaces, without the margin they share, and without the blank lines at the ends

#### Scenario: Agent asks to read the clipboard
- **WHEN** an agent sends an OSC 52 request to read the clipboard
- **THEN** nothing is sent back and the clipboard is unchanged

#### Scenario: Copy re-sent on opening a session
- **WHEN** the owner copies in session A, then in session B, then opens session A again, and Claude Code in A sends its old selection again as it redraws
- **THEN** the clipboard keeps B's text and no "Copied" note shows
