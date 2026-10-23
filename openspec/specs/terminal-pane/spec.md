# terminal-pane Specification

## Purpose
Shows a hosted agent session inside the Headquarter app with the fidelity of a normal terminal, so every native agent feature works there unchanged.

## Requirements

### Requirement: Faithful rendering
The system SHALL render a session's terminal output with its colours (16, 256, and truecolor), text attributes, wide characters, cursor shape and visibility, and alternate screen, as a standard xterm-compatible terminal would.

#### Scenario: Agent option picker
- **WHEN** an attached agent shows a multiple-choice question with a highlighted option
- **THEN** the pane shows the options, the highlight, and the cursor where the agent placed them

#### Scenario: Wide characters
- **WHEN** the agent prints CJK text or emoji
- **THEN** each takes two cells and the following text stays aligned

### Requirement: Keyboard input
The system SHALL send the owner's keys to the attached agent in xterm encoding, including Ctrl, Alt, and Shift combinations, arrows, navigation and function keys, and SHALL honour the modes the agent sets (application cursor, bracketed paste).

#### Scenario: Navigating a picker
- **WHEN** the owner presses the down arrow and Enter in an agent's option picker
- **THEN** the agent receives the same bytes a standard terminal would send and selects the next option

#### Scenario: Composed input
- **WHEN** the owner types a character composed with a dead key or an input method
- **THEN** the agent receives that character as UTF-8

### Requirement: Clipboard
The system SHALL copy selected text to the system clipboard and SHALL paste clipboard text to the agent, wrapped for bracketed paste when the agent enabled it.

#### Scenario: Paste into an agent prompt
- **WHEN** the owner pastes several lines while the agent has bracketed paste enabled
- **THEN** the agent receives the text between bracketed-paste markers

### Requirement: Scrollback and mouse
The system SHALL keep scrollback for the normal screen, SHALL send mouse clicks and wheel events to an agent that enabled mouse reporting, and SHALL otherwise use the mouse for selection and scrolling.

#### Scenario: Scrolling back
- **WHEN** the owner scrolls up on the normal screen of an agent that did not enable mouse reporting
- **THEN** earlier output is shown without sending anything to the agent

### Requirement: Size follows the pane
The system SHALL size the terminal to the pane and SHALL tell the host the new size whenever it changes.

#### Scenario: Window resized
- **WHEN** the owner resizes the window while attached
- **THEN** the agent redraws at the new number of columns and rows

### Requirement: Terminal replies
The system SHALL answer the agent's terminal queries (device attributes, cursor position) the way a standard terminal would.

#### Scenario: Cursor position query
- **WHEN** the agent asks for the cursor position
- **THEN** it receives the position report and continues without waiting

### Requirement: Session details header
The system SHALL show above every session in the terminal pane a one-line header, divided from the terminal by a line, with the session's title, full folder and git branch, and the model, context used and left, and cost when the session's agent reports them, leaving out anything the agent does not report.

#### Scenario: Claude Code session
- **WHEN** a Claude Code session has answered a message
- **THEN** its header shows the title, the folder, the branch, the model, the context used and left, and the cost

#### Scenario: Codex session
- **WHEN** a Codex session has finished a turn
- **THEN** its header shows the title, the folder, the branch, the model, and the context used and left, and no cost

#### Scenario: Antigravity session
- **WHEN** an Antigravity CLI session is shown
- **THEN** its header shows the title, the folder and the branch, with no empty or zero values in place of the details agy does not report to Headquarter

#### Scenario: Narrow pane
- **WHEN** the pane is too narrow for every item
- **THEN** the cost goes first, then the context left, the model and the branch, and the folder is shortened in the middle

#### Scenario: Token details
- **WHEN** the owner clicks the context meter of a Claude Code session
- **THEN** a popover lists the input, output and cache token counts, the window size and the session id

### Requirement: Details stay in memory
The system SHALL keep session details only in memory, SHALL NOT write them to the index or the log, and SHALL NOT read prompt or output text to obtain them.

#### Scenario: After a session
- **WHEN** a session with details ends and the index and log are inspected
- **THEN** they contain no token counts, model names, costs or context figures from it

### Requirement: Dropped files
The system SHALL type the paths of files and folders dropped on the terminal pane into the attached agent as a paste, each with shell characters escaped by a backslash, separated by spaces and followed by a space, and SHALL ignore drops anywhere else.

#### Scenario: Image onto a Claude Code prompt
- **WHEN** the owner drags `Screenshot 2026-10-23 at 14.29.43.png` from the Desktop onto a Claude Code session's pane
- **THEN** Claude Code receives `/Users/<owner>/Desktop/Screenshot\ 2026-10-23\ at\ 14.29.43.png ` as a bracketed paste and attaches the image

#### Scenario: Several files
- **WHEN** the owner drops two files on the pane at once
- **THEN** both paths are typed, separated by a space

#### Scenario: Dropped outside the pane
- **WHEN** the owner drops a file on the sidebar or on a pane with no attached session
- **THEN** nothing is typed

### Requirement: Line height from the font
The system SHALL space the pane's lines by the font's ascent plus descent, each rounded, as Alacritty does, and SHALL use 1.2 times the font size only when the font reports no metrics.

#### Scenario: JetBrains Mono at 14
- **WHEN** the settings file sets JetBrainsMono Nerd Font at size 14
- **THEN** the pane's lines are 18 points apart, as in Alacritty with the same settings
