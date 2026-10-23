# Spec Delta

## MODIFIED Requirements

### Requirement: Add sessions
The system SHALL offer an Open sessions window (the sidebar's command field, Cmd-O or Ctrl-Shift-O) that lists, by provider, every session not in the sidebar, newest first by day, with a session running in another terminal greyed in its place, tagged "In another terminal", and not pickable, because that terminal owns it. It SHALL add the sessions the owner picks to the sidebar in no group and show the first.

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
- **THEN** it is listed in date order, greyed, tagged "In another terminal", and cannot be picked

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
