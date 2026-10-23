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

### Requirement: Scrollback and mouse
The system SHALL keep scrollback for the normal screen, SHALL select text when the owner drags with the left button and copy it to the clipboard on release, except that a drag in a full-screen agent that enabled mouse reporting goes to that agent unless Shift is held, SHALL send clicks without a drag and wheel events to an agent that enabled mouse reporting, and SHALL otherwise use clicks to clear the selection and the wheel to scroll.

#### Scenario: Scrolling back
- **WHEN** the owner scrolls up on the normal screen of an agent that did not enable mouse reporting
- **THEN** earlier output is shown without sending anything to the agent

#### Scenario: Dragging in Claude Code's full-screen view
- **WHEN** the owner drags past the top of the pane in Claude Code's full-screen view, then releases
- **THEN** Claude Code receives the drag, scrolls and selects, its copy is on the clipboard, and the pane says "Copied"

#### Scenario: Dragging over an agent on the normal screen
- **WHEN** the owner drags across text in an agent on the normal screen, whether or not it enabled mouse reporting, then releases
- **THEN** Headquarter selects the text, the clipboard holds it, and nothing of the drag is sent to the agent

#### Scenario: Clicking in an agent that uses the mouse
- **WHEN** the owner clicks without moving in a session whose agent enabled mouse reporting
- **THEN** the agent receives the press and the release at that cell

#### Scenario: Word and line
- **WHEN** the owner double-clicks a word, or triple-clicks a line
- **THEN** the word or line is selected and copied

#### Scenario: Dragging past the bottom
- **WHEN** the owner drags from the pane to below its bottom edge on the normal screen
- **THEN** the view scrolls through the output while the selection follows, faster the further below the pointer is

#### Scenario: Readable selection
- **WHEN** the theme sets a selection text colour, as Catppuccin does
- **THEN** selected text is drawn in that colour on the selection background
