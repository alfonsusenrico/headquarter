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

### Requirement: Background and review look apart from working
The system SHALL show "background" in its own colour with two still dots, and "review" in another colour with a diamond, both apart from working's colour and movement and readable at 4.5:1 on the sidebar and the open row in every theme. Themes made from a palette SHALL take the colours from its cyan and magenta, and a theme file MAY set them.

#### Scenario: Side by side
- **WHEN** one session works, one shows "background" and one shows "review" in the built-in dark theme
- **THEN** "working" is blue with moving dots, "background" is teal with still dots, and "review" is violet with a diamond

#### Scenario: A template
- **WHEN** the owner uses the Nord template
- **THEN** "background" is Nord's cyan and "review" is Nord's magenta, each readable on the sidebar and the open row

#### Scenario: A theme file sets them
- **WHEN** the owner's theme file sets `background_work` in its interface colours
- **THEN** "background" uses that colour, and "review" keeps the colour made from the palette

## REMOVED Requirements

### Requirement: Background work counts as working
**Reason**: Background work shows as "background" beside a session at its prompt, not as working (owner, 2026-10-29: option E1). Claude Code 2.1.312 no longer keeps a session record for the sessions Headquarter launches, and its hooks and status reports name the work itself.
**Migration**: See "Background work beside the status" and "Background and review sort with working".

