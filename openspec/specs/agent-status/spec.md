# agent-status Specification

## Purpose
Tells the owner, for each agent session Headquarter launches, whether the agent is working, needs the owner, has finished (read or unread), or is inactive, using signals injected per launch and the session's terminal output.

## Requirements

### Requirement: Five session statuses
The system SHALL report every launched session as exactly one of: working, needs you, idle not read yet, idle (stored as working, needs-you, done-unread, done). A session whose agent is not running is stored as inactive and shown as inactive, or as starting while it waits to start with the host. When signals conflict, needs you SHALL take precedence over working, working over idle not read yet, and idle not read yet over idle.

#### Scenario: Permission request while working
- **WHEN** a session is working and the agent reports a permission request
- **THEN** the session's status becomes needs you

#### Scenario: Owner answers the request
- **WHEN** a session is needs you and the agent reports tool activity or a new prompt
- **THEN** the session's status becomes working

#### Scenario: Words shown
- **WHEN** a session's agent finishes a turn the owner has not seen
- **THEN** the sidebar, the rail tooltip and `hq ls` say "idle, not read yet", and `hq ls --json` still reports `done-unread`

#### Scenario: Agent not running
- **WHEN** a sidebar session's agent was quit and the host is not about to start it
- **THEN** the sidebar shows it as inactive and `hq ls` says "inactive"

### Requirement: Unread completion
The system SHALL mark a session idle not read yet when its agent completes a turn, and SHALL mark it idle only after the owner has seen it.

#### Scenario: Turn completes unattended
- **WHEN** the agent reports that a turn completed
- **THEN** the session's status becomes idle not read yet

#### Scenario: Owner looks at the session
- **WHEN** a session is idle not read yet and the owner interacts with it
- **THEN** the session's status becomes idle

### Requirement: Per-launch instrumentation only
The system SHALL pass any instrumentation (hooks, notify programs, status lines, session ids) to an agent through that launch's command-line arguments or environment, and SHALL NOT create, modify, or delete any agent configuration file.

#### Scenario: Claude session launched
- **WHEN** the owner launches a Claude Code session through the system
- **THEN** hooks and the session id are passed on the command line and no file under `~/.claude` or the project's `.claude/` changes

#### Scenario: Agent has no per-launch mechanism
- **WHEN** the owner launches an agent that accepts no per-launch instrumentation
- **THEN** the system launches it without instrumentation, derives status from terminal signals only, and writes no configuration file

### Requirement: Instrumentation never blocks the agent
The system's injected hook programs SHALL exit successfully, print nothing, and finish within one second whether or not the system is running.

#### Scenario: Probe not running
- **WHEN** an injected hook runs after the system has stopped listening
- **THEN** the hook exits with status 0 within one second without output, and the agent continues normally

### Requirement: Owner's own settings preserved
The system SHALL keep the owner's existing hooks, notify program, and status line working in sessions it launches.

#### Scenario: Owner has a custom status line
- **WHEN** the owner's Claude settings define a status line command and a session is launched through the system
- **THEN** the owner sees the same status line output as without the system

#### Scenario: Owner has a Codex notify program
- **WHEN** the owner's Codex configuration defines a notify program and a session is launched through the system
- **THEN** the owner's notify program still receives each turn-complete notification

### Requirement: Terminal signals
The system SHALL recognise status signals that agents emit in their terminal output (program-status, notification, and title sequences, and the bell) and SHALL pass the agent's output to the owner's terminal unchanged except for replies to support queries it answers itself.

#### Scenario: Agent sets a working title
- **WHEN** a launched agent emits a title sequence that indicates work in progress and no structured event says otherwise
- **THEN** the session's status becomes working

#### Scenario: Unrecognised sequence
- **WHEN** the agent emits an escape sequence the system does not recognise
- **THEN** the bytes reach the owner's terminal unchanged and the status does not change

### Requirement: No prompt or output persistence
The system SHALL NOT write prompt text, model output, or tool input values to any file, log, or store. Persisted session events SHALL contain only metadata: event kind, reason code, session id, working folder, tool name, agent, agent version, and timestamps.

#### Scenario: Prompt submitted
- **WHEN** a hook payload containing the owner's prompt is received
- **THEN** the recorded event contains the event kind and session id and does not contain the prompt text

#### Scenario: Malformed hook payload
- **WHEN** a hook delivers a payload that is not valid JSON or lacks a session id
- **THEN** the event is discarded with a metadata-only warning and the session's status is unchanged

### Requirement: Launched agents start as top-level sessions
The system SHALL launch every agent without the variables another agent sets for the commands it runs (its session id, child-session and sandbox markers), and SHALL keep every other variable of the environment it was started in.

#### Scenario: Headquarter started from inside Claude Code
- **WHEN** the host was started by a command run inside a Claude Code session and the owner launches a Claude session
- **THEN** the launched agent sees no `CLAUDE_CODE_CHILD_SESSION` or `CLAUDECODE` variable and saves its transcript

#### Scenario: Owner's agent settings
- **WHEN** the environment that started the host sets `CLAUDE_CODE_USE_BEDROCK`
- **THEN** the launched Claude session still receives it

### Requirement: Agents keep their native run mode
The system SHALL NOT pass options that change how an agent runs beyond its per-launch instrumentation, and SHALL launch Codex without configuration overrides.

#### Scenario: Codex with its shared background server
- **WHEN** the owner launches a Codex session through the system
- **THEN** Codex starts with no `-c` option, shows no embedded-mode warning, and its status is reported from OSC 7501

### Requirement: Inactive sessions
The system SHALL keep a sidebar session whose agent ends without the owner closing it (a quit from inside the agent, or an exit with an error) in the sidebar as inactive, SHALL keep a running session idle however long it waits, and SHALL start an inactive session's agent again only when the owner asks.

#### Scenario: Quit from inside
- **WHEN** the owner types `/exit` in a Claude Code session
- **THEN** the session stays in the sidebar as inactive and its agent's memory is freed

#### Scenario: Agent crashed
- **WHEN** a hosted agent exits with a non-zero code without the owner closing it
- **THEN** the session stays in the sidebar as inactive and is not restarted

#### Scenario: Waiting overnight
- **WHEN** a hosted session has been idle for twelve hours with its agent still running
- **THEN** its status is still idle

### Requirement: agy's artifact review
The system SHALL treat agy's report of an artifact waiting for review (`blocked`, kind permission, message "Approve plan") as the end of the turn, so the session shows unread until the owner looks, and SHALL keep agy's other blocked reports as needing the owner.

#### Scenario: A turn that leaves an artifact
- **WHEN** an agy turn ends and agy reports its artifact review
- **THEN** the session shows as done and unread, not needs you

#### Scenario: A real approval
- **WHEN** agy reports it is waiting for folder trust, a command approval, a question or sign-in
- **THEN** the session shows needs you

### Requirement: Status from the session's own conversation
The system SHALL change a Claude Code session's status, needs-you reason, and header details only from hook and status-line reports of that session's own conversation, and SHALL ignore reports from another conversation that carry the same session's launch environment. It SHALL follow the session to the conversation its own agent starts, including a resume that gives the conversation a new id and a `/clear` or `/compact` in place.

#### Scenario: A background Claude Code session waits
- **WHEN** a Claude Code process started by Claude Code's own daemon, with the session's launch environment, reports that it is waiting for input while the session works
- **THEN** the session stays working and its details are unchanged

#### Scenario: The owner clears the conversation
- **WHEN** the owner runs `/clear` in a session and Claude Code starts a new conversation in the same process
- **THEN** the session's status follows the new conversation's reports

#### Scenario: A resume with a new conversation id
- **WHEN** the host resumes a session and Claude Code continues it under a new conversation id
- **THEN** the session's status follows the reports of that new conversation

### Requirement: Background work beside the status
The system SHALL show "background" for a session at its prompt while work its agent started still runs (background agents, commands and monitors, from what the agent reports and the processes those commands started), with each kind's count and the newest agent's title in the session header, the Usage window and the phone's session header. Needs you SHALL always win, and an unknown or unreadable signal SHALL add nothing.

#### Scenario: Claude background work after the turn
- **WHEN** a Claude Code session ends its turn while a background agent and two background shell commands it started still run
- **THEN** its row says "background", and its header shows "1 agent" and "2 shells" with the agent's title

#### Scenario: The commands end
- **WHEN** those commands' processes exit and the agent finishes
- **THEN** the row shows its age and glyph again

#### Scenario: Helpers are not background work
- **WHEN** a session at its prompt has MCP servers or other helper processes running
- **THEN** the row shows its age, not "background"

#### Scenario: Needs you during background work
- **WHEN** a session with background work asks for permission
- **THEN** it shows needs you

### Requirement: Plans to review
The system SHALL show "review" for an agy session whose turn ended with a plan to review, and SHALL clear it when agy reports the next state or a new prompt starts a turn. It SHALL not count as needs you and SHALL not notify.

#### Scenario: A plan to review
- **WHEN** an agy turn ends with "Approve plan"
- **THEN** the row says "review", the header names the plan to review, and nothing notifies

#### Scenario: Rejected
- **WHEN** the owner rejects the plan
- **THEN** the row shows its age again

### Requirement: Background and review sort with working
Inside each group and the ungrouped list, sessions showing "background" or "review" SHALL sort with working sessions, after those needing the owner.

#### Scenario: Background rises
- **WHEN** a session in a group shows "background" and another in the same group finished more recently
- **THEN** the "background" session is listed above the finished one

### Requirement: Background and review look apart from working
The system SHALL show "background" in its own colour with a stacked-layers mark, and "review" in another colour with a diamond, both still, both apart from working's colour and movement and readable at 4.5:1 on the sidebar and the open row in every theme. Themes made from a palette SHALL take the colours from its cyan and magenta, and a theme file MAY set them.

#### Scenario: Side by side
- **WHEN** one session works, one shows "background" and one shows "review" in the built-in dark theme
- **THEN** "working" is blue with moving dots, "background" is teal with the stacked-layers mark, and "review" is violet with a diamond

#### Scenario: A template
- **WHEN** the owner uses the Nord template
- **THEN** "background" is Nord's cyan and "review" is Nord's magenta, each readable on the sidebar and the open row

#### Scenario: A theme file sets them
- **WHEN** the owner's theme file sets `background_work` in its interface colours
- **THEN** "background" uses that colour, and "review" keeps the colour made from the palette

#### Scenario: Background work after the turn
- **WHEN** a session at its prompt has a background agent running
- **THEN** its row ends with "background" and the stacked-layers mark where other rows show their glyph
