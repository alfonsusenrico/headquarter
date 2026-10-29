## ADDED Requirements

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
