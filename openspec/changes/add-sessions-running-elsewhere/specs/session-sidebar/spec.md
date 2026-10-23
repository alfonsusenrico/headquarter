# Spec Delta

## MODIFIED Requirements

### Requirement: Add sessions
The system SHALL offer an Open sessions window (the sidebar's command field, Cmd-O or Ctrl-Shift-O) that lists, by provider, every session not in the sidebar, newest first by day, with a session running in another terminal in its place and tagged "In another terminal". Such a session SHALL be pickable when its conversation is known, and greyed and not pickable otherwise. It SHALL add the sessions the owner picks to the sidebar in no group and show the first.

#### Scenario: Several sessions added
- **WHEN** the owner opens the window, picks three conversations and presses "Open 3"
- **THEN** all three appear in the sidebar's ungrouped list and the first is shown in the pane

#### Scenario: Already in the sidebar
- **WHEN** the owner opens the window
- **THEN** sessions already in the sidebar are not listed

#### Scenario: Finding a session
- **WHEN** the owner types part of a title in the Open sessions search
- **THEN** sessions whose titles contain it are listed, and a session already in the sidebar is not among them; it is found in the sidebar itself

#### Scenario: Running elsewhere
- **WHEN** a Claude session runs in another terminal with a known conversation
- **THEN** it is listed in date order, tagged "In another terminal", and can be picked

#### Scenario: Added while running elsewhere
- **WHEN** the owner picks a session running in another terminal and presses "Open 1"
- **THEN** it appears in the sidebar marked "ext" with its status, the pane says it is running in another terminal, and the other process is unaffected

#### Scenario: Picking keeps the place
- **WHEN** the owner scrolls halfway down and picks a session
- **THEN** the row shows as picked and the list neither re-sorts nor scrolls

#### Scenario: Filtered
- **WHEN** the owner chooses "Codex" and types part of a title
- **THEN** only Codex sessions whose titles contain that text are listed

#### Scenario: Folder search
- **WHEN** the owner types `~/wo`, presses Tab, then Enter
- **THEN** the field completes to `~/work/`, and Enter filters the list to sessions in `~/work` and its subfolders, shown as a folder chip

#### Scenario: Agent without sessions
- **WHEN** an enabled provider has no sessions outside the sidebar
- **THEN** its entry shows 0 and choosing it lists nothing

#### Scenario: Outside session without a conversation
- **WHEN** an agent runs in another terminal and no conversation is linked to it
- **THEN** it is listed greyed, tagged "In another terminal", cannot be picked, and never appears in the sidebar

#### Scenario: Unknown session from the command line
- **WHEN** the owner runs `hq add` naming one known session and one unknown name
- **THEN** the command fails naming the unknown one and the sidebar is unchanged

### Requirement: Opening sessions
The system SHALL show a sidebar session in the terminal pane when chosen, starting its agent again on the same conversation when it is not running, SHALL mark that row as the open one distinctly from the keyboard's place and from rows under the pointer, SHALL state that a session running in another terminal is observed only, and SHALL say so when such a session is closed in the other terminal while shown, starting nothing until the owner chooses it.

#### Scenario: Ended session
- **WHEN** the owner chooses a session whose agent is not running
- **THEN** the pane says "Opening…" and then shows the agent continuing that conversation, with no Resume step

#### Scenario: Starting fails
- **WHEN** the agent cannot be started again (for example its folder is gone)
- **THEN** the pane gives the reason and an "Open again" button

#### Scenario: Running elsewhere
- **WHEN** the owner chooses a session marked "ext"
- **THEN** the pane says it is running in another terminal, and Headquarter does not attach to or signal that process

#### Scenario: Closed in the other terminal
- **WHEN** a session marked "ext" is shown and the owner closes its agent in the other terminal
- **THEN** within one outside scan the pane says "Closed in the other terminal." with an "Open here" button, and no agent is started

#### Scenario: Continued here
- **WHEN** the owner presses "Open here" on a session closed in the other terminal
- **THEN** the pane says "Opening…" and then shows the agent continuing that conversation

#### Scenario: Which one is open
- **WHEN** the owner opens a session and then moves the keyboard selection to another row
- **THEN** the open session's row stays filled with a bar on its edge, and the other row shows only an outline
