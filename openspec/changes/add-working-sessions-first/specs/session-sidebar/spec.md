## MODIFIED Requirements

### Requirement: Sidebar sections
The system SHALL show, from top to bottom, the "New group" line, a "Needs you" section only while a sidebar session needs the owner, the owner's groups in the owner's order, and the sidebar sessions in no group under an "Ungrouped" label only when there are any. Each group header SHALL show the group's session count as "1 session" or "N sessions". A session needing the owner SHALL be listed both in "Needs you" and in its group or the ungrouped list. Inside each group and the ungrouped list, sessions needing the owner SHALL come first, then working sessions, then the rest, each of these by most recent activity first. A session running in another terminal SHALL be among the rest.

#### Scenario: Nothing needs the owner
- **WHEN** no sidebar session needs the owner
- **THEN** the sidebar has no "Needs you" section and starts with the first group

#### Scenario: Permission prompt in a group
- **WHEN** a session in the group "Backend" asks for permission
- **THEN** it is listed in "Needs you" and first under "Backend", and the "Backend" header still shows "3 sessions" without a needs-you count

#### Scenario: Answered
- **WHEN** the owner answers that prompt and the agent resumes work
- **THEN** the session leaves "Needs you" and stays under "Backend" showing working, above the sessions that are not working

#### Scenario: Working for a while
- **WHEN** a session in "Personal project" has been working for 20 minutes and another in the same group finished 17 minutes ago
- **THEN** the working session is listed above the finished one, and the groups keep the owner's order

#### Scenario: A turn ends
- **WHEN** a working session finishes its turn while other sessions in its group are still working
- **THEN** it is listed below them and above the group's sessions with older activity

#### Scenario: No groups yet
- **WHEN** the owner has made no groups
- **THEN** the sidebar sessions are one list under the "Ungrouped" label: needing the owner, then working, then the rest, each by most recent activity first

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

## ADDED Requirements

### Requirement: Rows hold still while in use
While the pointer is over the sidebar or the icon rail, or a session or group is being dragged, the system SHALL keep each listed session in its place within its list and SHALL still update its status, word, glyph and age. A session new to a list SHALL take its place by the sidebar's order. When the pointer leaves and no drag is under way, the system SHALL apply the sidebar's order at once.

#### Scenario: A session starts working under the pointer
- **WHEN** the pointer rests on the sidebar and a session lower in its group starts working
- **THEN** its row shows working in its current place, and moves to the top of the group when the pointer leaves the sidebar

#### Scenario: Pointer elsewhere
- **WHEN** the pointer is over the terminal pane and a session starts working
- **THEN** its row moves to the top of its group at once

#### Scenario: Moved by the owner during the hold
- **WHEN** the pointer is on the sidebar and the owner drags a working session into another group
- **THEN** it is listed in that group at the place the sidebar's order gives it

#### Scenario: Menu on a moving row
- **WHEN** the owner opens a session's menu and the session's place changes before an item is chosen
- **THEN** the chosen item acts on that session
