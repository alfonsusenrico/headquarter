## MODIFIED Requirements

### Requirement: Opening sessions
The system SHALL show a sidebar session in the terminal pane when chosen; for an inactive session it SHALL show the session with a "Resume Session" button and start its agent on the same conversation only when it is pressed, or offer "Remove from Sidebar" instead when its folder no longer exists. It SHALL mark the open row distinctly from the keyboard's place and from rows under the pointer, and SHALL state that a session running in another terminal is observed only.

#### Scenario: Ended session
- **WHEN** the owner chooses an inactive session
- **THEN** the pane shows its title, folder, "Inactive" and a "Resume Session" button, and no agent starts

#### Scenario: Resumed
- **WHEN** the owner presses "Resume Session", or Enter while the pane has the keyboard
- **THEN** the pane says "Opening…" and then shows the agent continuing that conversation

#### Scenario: Starting fails
- **WHEN** the agent cannot be started (for example its executable is missing)
- **THEN** the session stays inactive and the pane gives the reason above "Resume Session"

#### Scenario: Folder gone
- **WHEN** the owner opens an inactive session whose folder no longer exists
- **THEN** the pane says the folder no longer exists and offers "Remove from Sidebar", which asks before removing it, in place of "Resume Session"

#### Scenario: Running elsewhere
- **WHEN** the owner chooses a session marked "ext"
- **THEN** the pane says it is running in another terminal, and Headquarter does not attach to or signal that process

#### Scenario: Closed in the other terminal
- **WHEN** a session marked "ext" is shown and the owner closes its agent in the other terminal
- **THEN** within one outside scan the session becomes inactive, the pane shows "Resume Session", and no agent is started

#### Scenario: Continued here
- **WHEN** the owner presses "Resume Session" on a session closed in the other terminal
- **THEN** the pane says "Opening…" and then shows the agent continuing that conversation in Headquarter

#### Scenario: Which one is open
- **WHEN** the owner opens a session and then moves the keyboard selection to another row
- **THEN** the open session's row stays filled with a bar on its edge, and the other row shows only an outline

### Requirement: Status by shape
The system SHALL mark needs you, idle not read yet, and idle with glyphs that differ in shape as well as colour: an amber ring, a filled speech bubble with a dot, and an outlined speech bubble. It SHALL mark working with "working" fading in and out and no glyph. It SHALL mark inactive with a crescent moon and a dimmed title and age, and a session starting with the host with "starting" fading in and out. It SHALL show idle-not-read-yet sessions with a bold title until the owner has seen them.

#### Scenario: Finished while away
- **WHEN** a sidebar session finishes a turn while another session is shown
- **THEN** its row shows the filled speech bubble with a dot and a bold title

#### Scenario: Seen
- **WHEN** the owner opens that session in the terminal pane with the app focused
- **THEN** it becomes idle, with the muted speech bubble and a regular title

#### Scenario: Finished while watched
- **WHEN** the session shown in the focused pane finishes a turn
- **THEN** it becomes idle without passing through idle not read yet

#### Scenario: Working
- **WHEN** a sidebar session is working
- **THEN** its row shows "working" fading in and out where the age is, and no glyph

#### Scenario: Inactive
- **WHEN** a sidebar session's agent was quit
- **THEN** its row shows a crescent moon where the status glyph goes, with its title and age dimmed

#### Scenario: Starting with the host
- **WHEN** the host starts with three sessions to start
- **THEN** all three rows show "starting" fading in and out where the age is, with no glyph, and each switches to its own status as its agent becomes ready, one after another

## ADDED Requirements

### Requirement: Removing an inactive session
The system SHALL remove an inactive session from the sidebar with the ✕ on its row after the owner confirms "Remove “title” from the sidebar?", keeping it in the index so Add sessions can offer it again.

#### Scenario: Removed
- **WHEN** the owner presses the ✕ of an inactive session and confirms
- **THEN** the session leaves the sidebar and Add sessions offers it again

#### Scenario: Remove cancelled
- **WHEN** the owner presses the ✕ of an inactive session and cancels
- **THEN** the session stays in the sidebar, inactive
