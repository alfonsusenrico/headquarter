## MODIFIED Requirements

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
