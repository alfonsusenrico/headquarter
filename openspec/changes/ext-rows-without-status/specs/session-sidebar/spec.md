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
- **THEN** it appears in the sidebar marked "ext" with its age, the pane says it is running in another terminal, and the other process is unaffected

#### Scenario: Rows line up
- **WHEN** rows that can and cannot be picked are listed together
- **THEN** their tags and ages line up in the same columns

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

### Requirement: Sidebar sections
The system SHALL show, from top to bottom, the "New group" line, a "Needs you" section only while a sidebar session needs the owner, the owner's groups in the owner's order, and the sidebar sessions in no group under an "Ungrouped" label only when there are any. Each group header SHALL show the group's session count as "1 session" or "N sessions". A session needing the owner SHALL be listed both in "Needs you" and in its group or the ungrouped list. A session running in another terminal SHALL show no status there: its row shows "ext" and its age, it is never listed in "Needs you", and its status returns once it runs in Headquarter.

#### Scenario: Nothing needs the owner
- **WHEN** no sidebar session needs the owner
- **THEN** the sidebar has no "Needs you" section and starts with the first group

#### Scenario: Permission prompt in a group
- **WHEN** a session in the group "Backend" asks for permission
- **THEN** it is listed in "Needs you" and under "Backend", and the "Backend" header still shows "3 sessions" without a needs-you count

#### Scenario: Answered
- **WHEN** the owner answers that prompt and the agent resumes work
- **THEN** the session leaves "Needs you" and stays under "Backend" showing working

#### Scenario: No groups yet
- **WHEN** the owner has made no groups
- **THEN** the sidebar sessions are one list under the "Ungrouped" label, most recent activity first

#### Scenario: Every session in a group
- **WHEN** groups exist and no sidebar session is ungrouped
- **THEN** no "Ungrouped" label is shown

#### Scenario: Dragging out with nothing ungrouped
- **WHEN** every session is in a group and the owner drags one
- **THEN** an "Ungrouped" drop area appears for the length of the drag, and dropping there takes the session out of its group

#### Scenario: Row content
- **WHEN** a sidebar session is listed
- **THEN** its single row shows the agent's mark, its title, its age or status word, and its status glyph; a session running in another terminal shows the mark, the title, "ext" and its age only

#### Scenario: Folder on hover
- **WHEN** the pointer rests on a sidebar row
- **THEN** a tooltip shows only the session's full folder, with the home folder as `~`

#### Scenario: Working in another terminal
- **WHEN** a sidebar session's agent is working, or asks for permission, in another terminal
- **THEN** its row shows "ext" and its age with no status word or glyph, and it is not listed in "Needs you"

#### Scenario: Continued here
- **WHEN** that session is closed in the other terminal and the owner opens it in Headquarter
- **THEN** its row drops "ext" and shows its status again
