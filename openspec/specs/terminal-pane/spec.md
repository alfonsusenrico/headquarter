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
The system SHALL send the owner's keys to the attached agent in xterm encoding, including Ctrl, Alt, and Shift combinations, Tab and Shift+Tab, arrows, navigation and function keys, and SHALL honour the modes the agent sets (application cursor, bracketed paste). Keys the terminal sends SHALL take precedence over the window's focus navigation while the terminal is focused.

#### Scenario: Navigating a picker
- **WHEN** the owner presses the down arrow and Enter in an agent's option picker
- **THEN** the agent receives the same bytes a standard terminal would send and selects the next option

#### Scenario: Composed input
- **WHEN** the owner types a character composed with a dead key or an input method
- **THEN** the agent receives that character as UTF-8

#### Scenario: Cycling an agent's mode
- **WHEN** the owner presses Shift+Tab in a focused terminal running Claude Code
- **THEN** the agent receives `ESC [ Z` and cycles its mode, and focus stays on the terminal

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
The system SHALL show above every session in the terminal pane a one-line header, 38 px tall and divided from the terminal by a line, with the session's title in a larger type than its details, its full folder and git branch, and the model, context used and left, and cost when the session's agent reports them, leaving out anything the agent does not report.

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
- **THEN** the cost goes first, then the context left, the model and the branch, and the folder is shortened in the middle while the title keeps its width

#### Scenario: Token details
- **WHEN** the owner clicks the context meter of a Claude Code session
- **THEN** a popover lists the input, output and cache token counts, the window size and the session id

#### Scenario: Context meter fill
- **WHEN** a Claude Code session has used 96% of its context window
- **THEN** the header's context meter fills 96% of its track, and a session at or over 100% fills it completely

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

### Requirement: Links
The system SHALL underline a web address (`http` or `https`) under the pointer with a hand pointer and open it in the default browser on a plain click, without sending that click to the agent, in the agent pane and the shell panel. It SHALL find links an agent marks up (OSC 8) and plain-text addresses, and SHALL NOT open other kinds of links.

#### Scenario: Plain address
- **WHEN** the shell prints `see https://example.com/docs.` and the owner clicks the address
- **THEN** `https://example.com/docs` opens in the browser, without the full stop, and the shell receives nothing

#### Scenario: Wrapped link from an agent
- **WHEN** Claude Code prints a link it wraps over two rows and the owner hovers the second row
- **THEN** both rows of the link are underlined and a click opens the full address

#### Scenario: Not a web link
- **WHEN** an agent marks up a `file:` link and the owner clicks it
- **THEN** nothing opens and the click goes where a click on plain text goes

#### Scenario: Dragging from a link
- **WHEN** the owner presses on a link and drags across the text beside it
- **THEN** the text is selected and copied as with any drag, and the link does not open

### Requirement: Agents told to mark up links
The system SHALL start agents and session shells with `FORCE_HYPERLINK=1`, so programs that check it mark up their links and a link they wrap over several rows opens whole.

#### Scenario: Claude Code marks up its links
- **WHEN** a Claude Code session prints a long address that wraps
- **THEN** its rows carry the marked-up link with the full address

### Requirement: Attach at the pane's size
The app SHALL attach to a session, or its shell, at the size its pane is laid out with, and SHALL NOT have the agent draw at a size the pane does not show. When the pane has no size 500 ms after opening, it SHALL attach at 80 columns and 24 rows.

#### Scenario: Coming back to a session
- **WHEN** the owner clicks a session in the sidebar that the window showed earlier
- **THEN** the app attaches at the pane's columns and rows, and the agent draws only at that size

#### Scenario: Coming back while the agent works
- **WHEN** the owner comes back to a session whose agent is producing output
- **THEN** the pane shows the agent's current screen with no text from another size
