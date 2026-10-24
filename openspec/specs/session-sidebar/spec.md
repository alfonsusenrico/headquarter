# session-sidebar Specification

## Purpose
Keeps the sessions the owner chose in view, in groups the owner makes with those needing them first and each status told by shape, and lets the owner add, open, rename, close and start them from the app or the command line.

## Requirements

### Requirement: Curated sidebar
The system SHALL show in the sidebar only sessions the owner has added and sessions started in Headquarter, SHALL keep them until the owner removes them, and SHALL never change an agent's history when a session is removed.

#### Scenario: First open
- **WHEN** the owner opens the app for the first time and has past agent conversations but has never started a session in Headquarter
- **THEN** the sidebar is empty and offers the "+" button to add sessions

#### Scenario: Session started in Headquarter
- **WHEN** the owner starts a session from the app or with `hq new`
- **THEN** it appears in the sidebar without being added

#### Scenario: Added session goes quiet
- **WHEN** an added session has been inactive for months
- **THEN** it is still in the sidebar, shown as inactive

#### Scenario: Session removed
- **WHEN** the owner removes a session from the sidebar
- **THEN** it leaves the sidebar, the agent's history is unchanged, and the session is still offered in "Add sessions"

#### Scenario: Restart
- **WHEN** the host or the app restarts
- **THEN** the sidebar holds the same sessions

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

### Requirement: Session actions
The system SHALL let the owner rename a sidebar session by clicking the title of the session open in the pane, and close a session with an ✕ shown at the right end of its row while the pointer is over it, and SHALL offer no right-click menu, no double-click action and no hover text on sidebar rows or group headers. A name the owner gives SHALL show instead of the agent's title.

#### Scenario: End session
- **WHEN** the owner presses the ✕ of a working hosted session and confirms in the dialog
- **THEN** the session leaves the sidebar at once and its agent is stopped in the background

#### Scenario: End cancelled
- **WHEN** the owner presses the ✕ of a working hosted session and cancels
- **THEN** the agent keeps running

#### Scenario: Close an idle session
- **WHEN** the owner presses the ✕ of an idle session
- **THEN** a dialog asks "Close “title”?", and once confirmed the session leaves the sidebar and the row below it is shown

#### Scenario: Close appears smoothly
- **WHEN** the pointer moves onto a row
- **THEN** the status glyph and age slide left over about 150 ms and the ✕ fades in at the right end, and they return when the pointer leaves

#### Scenario: Rename the open session
- **WHEN** the session "Greeting in one word" is open and the owner clicks its title in the sidebar, types "Smoke test" and presses Enter
- **THEN** the sidebar row and `hq ls` show "Smoke test"

#### Scenario: Rename cancelled
- **WHEN** the owner presses Escape in a session's name field
- **THEN** the original name is shown again and nothing is saved

#### Scenario: First click opens
- **WHEN** the owner clicks the title of a session that is not open
- **THEN** the session opens and no name field appears

#### Scenario: Right-click
- **WHEN** the owner right-clicks a sidebar row or a group header
- **THEN** no menu opens and nothing changes

#### Scenario: No hover text
- **WHEN** the pointer rests on a sidebar row
- **THEN** no tooltip appears; the folder is shown in the session header

### Requirement: Keyboard navigation
The system SHALL let the owner move through the sidebar, open sessions, collapse groups, open "Add sessions", and jump to the next session needing them using the keyboard alone, without taking keys the agents use. No key SHALL close or remove a session.

#### Scenario: Jump to the next session needing you
- **WHEN** two sessions need the owner and the owner presses the jump shortcut twice
- **THEN** the first and then the second of them is shown in the terminal pane

#### Scenario: Linux shortcuts
- **WHEN** the owner presses Ctrl-N or Ctrl-J in a terminal pane on Linux
- **THEN** the agent receives the key and the app takes no action

#### Scenario: Delete pressed in the sidebar
- **WHEN** a session is selected in the sidebar and the owner presses Delete or Backspace
- **THEN** nothing is closed or removed

### Requirement: Sidebar from the command line
The system SHALL list the sidebar with `hq ls`, every session with `hq ls --all`, and change the sidebar with `hq add` and `hq remove`, matching what the app shows.

#### Scenario: Empty sidebar
- **WHEN** the owner runs `hq ls` and the sidebar is empty
- **THEN** it says the sidebar is empty and names `hq ls --all` and `hq add`

#### Scenario: Same in both
- **WHEN** the owner adds a session with `hq add` while the app is open
- **THEN** the app's sidebar shows it within a second

### Requirement: Resizable sidebar
The system SHALL let the owner resize the sidebar by dragging the divider, with the sidebar following the pointer from 480 pixels down to the rail's width, SHALL keep a full sidebar between 180 and 480 pixels, and SHALL keep the width across restarts.

#### Scenario: Dragged wider
- **WHEN** the owner drags the divider 60 pixels to the right from the default width
- **THEN** the sidebar is 340 pixels wide and the terminal pane narrower, and after a restart it is still 340 pixels wide

#### Scenario: Limits
- **WHEN** the owner drags the divider past 480 pixels
- **THEN** the sidebar stays 480 pixels wide

#### Scenario: Dragged down to the rail
- **WHEN** the owner drags the divider from 280 pixels to 100 pixels
- **THEN** the sidebar narrows continuously under the pointer, is drawn as the rail once narrower than 180 pixels, and on release eases to the 56 pixel rail

### Requirement: Icon rail
The system SHALL collapse the sidebar to an icon rail when it is dragged narrower than 180 pixels, when the divider is double-clicked, or with Cmd-B (Ctrl-Shift-B on Linux), and SHALL show in the rail every sidebar session as its agent's mark alone, centred, with an amber ring when it needs the owner, sessions needing the owner first, collapsed groups as chips, and the usage ring nearest its limit, which opens every agent's usage.

#### Scenario: Snapped to the rail
- **WHEN** the owner drags the divider to 150 pixels and releases it
- **THEN** the sidebar becomes the icon rail, and a restart keeps the rail

#### Scenario: Needs you in the rail
- **WHEN** a session needs the owner while the rail is shown
- **THEN** its mark is listed first with an amber ring, and hovering it shows its title and folder

#### Scenario: Usage from the rail
- **WHEN** the owner clicks the rail's usage ring
- **THEN** the usage details of every enabled agent open beside the rail

#### Scenario: Back to full
- **WHEN** the owner presses Cmd-B while the rail is shown
- **THEN** the full sidebar returns at its last width

#### Scenario: Linux shortcut
- **WHEN** the owner presses Ctrl-B in a terminal pane on Linux
- **THEN** the agent receives the key and the sidebar does not change

### Requirement: Modals keep their state
The system SHALL keep clicks on or around an open modal or dialog from reaching the sidebar behind it, and SHALL keep the ticks and search of an open "Add sessions" when it is opened again.

#### Scenario: Click outside the list
- **WHEN** the owner ticks three sessions in "Add sessions" and clicks an empty part of the modal or the dimmed area around it
- **THEN** the three sessions stay ticked

#### Scenario: Opened again
- **WHEN** "Add sessions" is open with two sessions ticked and the owner presses ⌘O
- **THEN** the same modal stays open with both still ticked

### Requirement: Sidebar sections
The system SHALL show, from top to bottom, the "New group" line, a "Needs you" section only while a sidebar session needs the owner, the owner's groups in the owner's order, and the sidebar sessions in no group under an "Ungrouped" label only when there are any. Each group header SHALL show the group's session count as "1 session" or "N sessions". A session needing the owner SHALL be listed both in "Needs you" and in its group or the ungrouped list.

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

### Requirement: Owner-made groups
The system SHALL let the owner create, rename, reorder, collapse and delete groups in the app. A new group SHALL be offered with the name "New Group #n", with n the smallest number not used by such a name, ready to edit in place, and SHALL be created only when its name is saved. Group names SHALL be trimmed and 1 to 60 characters long.

#### Scenario: First group
- **WHEN** the owner chooses "New group" and there are no groups
- **THEN** a header appears after the last group with "New Group #1" selected in an inline field, while the "New group" line stays at the top

#### Scenario: Name saved by clicking outside
- **WHEN** the owner types "Backend" in the field and clicks elsewhere
- **THEN** the group is named "Backend"

#### Scenario: Blank name
- **WHEN** the owner clears the field of a new group and presses Enter
- **THEN** the group keeps the name "New Group #1"

#### Scenario: New group cancelled
- **WHEN** the owner chooses "New group" and presses Escape in the name field
- **THEN** no group is created and the sidebar is as it was

#### Scenario: Numbering reuses gaps
- **WHEN** groups "New Group #1" and "New Group #3" exist and the owner adds a group
- **THEN** it is named "New Group #2"

#### Scenario: Overlong name
- **WHEN** a client asks the host to name a group with 61 characters
- **THEN** the request fails saying names are at most 60 characters and nothing changes

#### Scenario: Reordered
- **WHEN** the owner drags the "Docs" header above the "Backend" header
- **THEN** "Docs" is listed before "Backend", also after a restart

### Requirement: Sessions moved by dragging
The system SHALL let the owner put a sidebar session in a group by dragging its row onto the group, SHALL light up a group while a session is dragged over it, and SHALL take a session out of its group when it is dropped on the ungrouped list. Each session SHALL be in at most one group.

#### Scenario: Dropped on a group
- **WHEN** the owner drags an ungrouped session over the group "Backend"
- **THEN** the "Backend" block lights up, and when the session is dropped there it is listed under "Backend" and no longer in the ungrouped list

#### Scenario: Dropped on a collapsed group
- **WHEN** the owner drops a session on the header of a collapsed group
- **THEN** the session joins that group, the group stays collapsed, and its count goes up by one

#### Scenario: Out of a group
- **WHEN** the owner drags a session from "Backend" onto the "Ungrouped" label
- **THEN** the session is listed in the ungrouped list

#### Scenario: Dropped on New group
- **WHEN** the owner drops a session on "New group"
- **THEN** a name field for a new group opens, and once the name is saved the group is created containing that session

#### Scenario: Short drag
- **WHEN** the owner presses a row and releases it after moving the pointer less than 2 pixels
- **THEN** the session opens as for a click and no group changes

### Requirement: Deleting a group
The system SHALL ask for confirmation before deleting a group and, once confirmed, SHALL move its sessions to the ungrouped list without ending or removing any of them.

#### Scenario: Delete confirmed
- **WHEN** the owner chooses "Delete group…" on a group with three sessions and confirms
- **THEN** the group is gone and its three sessions are in the ungrouped list, still running if they were

#### Scenario: Delete cancelled
- **WHEN** the owner chooses "Delete group…" and presses Escape
- **THEN** the group and its sessions are unchanged

### Requirement: Where new sessions go
The system SHALL put every new session, and every session added from Open sessions, in no group; the owner moves sessions into groups by dragging. A session removed from the sidebar SHALL leave its group.

#### Scenario: New session here
- **WHEN** the owner looks for "New session here" on the group "Docs"
- **THEN** there is none; a new session is listed ungrouped and the owner drags it into "Docs"

#### Scenario: Started from the command line
- **WHEN** the owner runs `hq new claude`
- **THEN** the session is listed in the ungrouped list

#### Scenario: Removed and added again
- **WHEN** the owner removes a session that was in "Docs" from the sidebar and later adds it again
- **THEN** it is listed in the ungrouped list

### Requirement: Collapsible groups
The system SHALL let the owner collapse and expand groups and SHALL remember collapsed groups across restarts, while still counting what needs the owner.

#### Scenario: Collapsed group needs the owner
- **WHEN** a group is collapsed and one of its sessions starts needing the owner
- **THEN** that session appears in the "Needs you" section, the collapsed header shows an amber dot that fades in and out and an amber bar on its left edge, and the header's tooltip says "1 needs you"

#### Scenario: Reduced motion
- **WHEN** the operating system asks for reduced motion and a collapsed group needs the owner
- **THEN** the amber dot and bar are shown without fading

#### Scenario: Expanded or answered
- **WHEN** the owner expands that group or answers the session
- **THEN** the dot and bar are gone

### Requirement: New session window
The system SHALL offer a New session window, opened with Cmd-N or Ctrl-Shift-N, in the same frame as Open sessions, with "Providers" on the left and a folder field with the folder browser below it, and SHALL start a session with the chosen provider's own command in the chosen folder, in no group, shown at once. The sidebar SHALL have no new-session form.

#### Scenario: Start in a typed folder
- **WHEN** the owner presses Cmd-N, types `~/code/project/`, chooses Codex and presses Enter on "Open here"
- **THEN** the window closes and a new Codex session in `~/code/project` is shown in the pane and listed ungrouped in the sidebar

#### Scenario: Folder missing
- **WHEN** the owner starts a session in a folder that no longer exists
- **THEN** the window stays open and names the missing folder

### Requirement: Folder browser
The system SHALL list folder names only under the folder field: for a typed path, that folder first and then its subfolders; for a bare name, the folders of that name at any depth under the home folder, by full path. Hidden folders SHALL be left out, and the keys and the pointer SHALL move into and out of folders.

#### Scenario: Into a subfolder
- **WHEN** the owner highlights `headquarter` in the list and presses Tab
- **THEN** the field reads `~/code/project/headquarter/`, that folder is the first row, tagged "Open here", and its subfolders follow

#### Scenario: Start only from "Open here"
- **WHEN** a subfolder is highlighted
- **THEN** Start session is disabled, and Enter goes into the subfolder instead of starting

#### Scenario: Found by name
- **WHEN** the owner types `team-flow-planner` and `~/code/project/acme/team-flow-planner` exists
- **THEN** it is listed by its full path, and choosing it shows that folder with its subfolders

#### Scenario: Hidden folders
- **WHEN** a folder contains `.git` and `src`
- **THEN** only `src` is listed

### Requirement: Agent marks
The system SHALL show each agent's official mark, fetched once from the vendor's published file onto the owner's machine and never bundled, beside its sessions in the sidebar, in the modal and in the usage footer, unaltered in shape. It SHALL show Headquarter's own drawn mark for an agent whose official mark is unavailable, and for every agent when the owner chooses drawn marks. Status SHALL still be told by the status glyph's shape, never by the mark.

#### Scenario: First start
- **WHEN** the app starts for the first time with network access, in dark mode
- **THEN** it fetches each hostable agent's mark once and keeps it unchanged in the data folder, and shows Codex and Antigravity in their one-colour marks in white and Claude Code in its orange spark

#### Scenario: Coloured marks chosen
- **WHEN** the settings file has `agent_marks = "color"` under `[sidebar]`
- **THEN** every agent shows its vendor's coloured mark

#### Scenario: Mark unavailable
- **WHEN** an agent's mark cannot be fetched, or the fetched file is not a PNG or SVG
- **THEN** nothing is kept, its sessions show the drawn mark, and the fetch is tried again at the next start

#### Scenario: Drawn marks chosen
- **WHEN** the settings file has `agent_marks = "drawn"` under `[sidebar]`
- **THEN** nothing is fetched and every agent shows its drawn mark

### Requirement: One-colour marks
The system SHALL by default use the vendor's own one-colour mark where one is published, drawn in black on a light theme and white on a dark one, and the vendor's coloured mark otherwise, and SHALL let the owner choose coloured marks for every agent.

#### Scenario: Light mode
- **WHEN** the system switches to light mode while one-colour marks are shown
- **THEN** the Codex and Antigravity marks are drawn in black and the Claude Code spark keeps its colour

### Requirement: Group headers edited in place
The system SHALL let the owner rename a group by clicking its name, which becomes a name field (a text cursor shows over the name), and delete a group with an ✕ shown on its header while the pointer is over it, after confirmation. Enter or a click elsewhere SHALL save the name, Escape SHALL restore the original, and a blank name SHALL take "New Group #n". Clicking elsewhere on the header SHALL collapse or expand the group.

#### Scenario: Rename by clicking the name
- **WHEN** the owner clicks the name "Backend", types "API" and clicks elsewhere
- **THEN** the group is named "API"

#### Scenario: Rename reverted
- **WHEN** the owner clicks the name "Backend", types "API" and presses Escape
- **THEN** the group is still named "Backend"

#### Scenario: Delete from the header
- **WHEN** the owner hovers the "Backend" header, presses its ✕ and confirms
- **THEN** the group is gone and its sessions are in the ungrouped list

#### Scenario: Collapse
- **WHEN** the owner clicks the "Backend" header outside its name and ✕
- **THEN** the group collapses

### Requirement: Motion within a frame budget
The app SHALL draw the sidebar's motion, a working row's "working" word and the waiting dot of a collapsed group, at most 12 times a second and only while such motion is on screen, and SHALL hold both still when the operating system asks for reduced motion.

#### Scenario: A session works
- **WHEN** a session in the sidebar is working and nothing else on screen changes
- **THEN** its "working" word fades between 35% and full strength every 2 s and the app draws at most 12 frames a second for it

#### Scenario: Nothing in motion
- **WHEN** no session on screen is working and no collapsed group is waiting on the owner
- **THEN** the app draws no frames for motion

#### Scenario: Reduced motion
- **WHEN** the operating system asks for reduced motion and a session is working
- **THEN** the "working" word is drawn still and the app draws no frames for it

### Requirement: Folder search leaves media libraries alone
The system SHALL NOT read the Music, Movies or Pictures folders directly in the home folder, nor package folders such as apps and media libraries, while it searches for project folders.

#### Scenario: Opening the New session window
- **WHEN** the owner opens the New session window on macOS
- **THEN** macOS does not ask whether Headquarter may access Apple Music, the photo library or videos

#### Scenario: A project's own media folder
- **WHEN** a project folder contains a folder named `Music`
- **THEN** that folder is still found by name

### Requirement: Add sessions offers only continuable conversations
The system SHALL leave out of Add sessions, and out of its counts, every conversation the host does not offer and every conversation whose working folder no longer exists, SHALL keep a session whose folder is unknown, and SHALL NOT delete such conversations from any agent's history or from its index.

#### Scenario: Conversation from a removed temporary folder
- **WHEN** an agy conversation was run in a temporary folder that has since been removed
- **THEN** Add sessions does not list it, and `hq ls --all` still does

#### Scenario: Folder still there
- **WHEN** a conversation's folder exists
- **THEN** Add sessions lists it as before

#### Scenario: Not offered by the host
- **WHEN** the host reports a conversation as not offered, such as a `codex exec` run or one Claude Code deleted
- **THEN** Add sessions does not list it, for every agent alike

### Requirement: Removing past conversations
The system SHALL offer, on each Add sessions row whose conversation is not running, a menu to remove it from its agent the way that agent allows: Claude Code's transcript and the folder beside it to the system Trash, Codex through its own archive or permanent delete, and nothing for agy; SHALL let Move to Trash and Archive be undone for 5 seconds before they take effect, SHALL ask before Delete permanently, and SHALL keep the menu's space on rows without one so their ages line up.

#### Scenario: Claude conversation to the Trash
- **WHEN** the owner chooses Move to Trash on a past Claude Code conversation and lets the toast pass
- **THEN** its transcript and the folder beside it are in the Trash, and the row is gone from Add sessions

#### Scenario: Undo
- **WHEN** the owner chooses Archive on a Codex conversation and presses Undo within 5 seconds
- **THEN** the row comes back and Codex is not called

#### Scenario: Permanent delete
- **WHEN** the owner chooses Delete permanently on a Codex conversation
- **THEN** a confirmation says it cannot be undone, and only after Delete does Headquarter run `codex delete --force` with its id

#### Scenario: agy and running sessions
- **WHEN** a row is an agy conversation, or a session running here or in another terminal
- **THEN** it has no menu, and its age lines up with the other rows

#### Scenario: Agent refuses
- **WHEN** Codex fails to archive or delete
- **THEN** the row comes back with a note that it could not be removed, and the host log names the agent and the way only

### Requirement: No status for sessions running elsewhere
The system SHALL show a sidebar session running in another terminal with its mark, title, "ext" and age only, SHALL NOT list it in "Needs you" or count it as needing the owner, and SHALL show its status again once it runs in Headquarter.

#### Scenario: Working in another terminal
- **WHEN** a sidebar session's agent is working, or asks for permission, in another terminal
- **THEN** its row shows "ext" and its age with no status word or glyph, and it is not listed in "Needs you"

#### Scenario: Continued here
- **WHEN** that session is closed in the other terminal and the owner opens it in Headquarter
- **THEN** its row drops "ext" and shows its status again

### Requirement: Group colours
The system SHALL give each group a colour from a soft eight-colour palette suited to the light or dark mode, shown as a dot before the group's name and as a thin line along its rows; SHALL choose it automatically from the group's identity until the owner picks one from the dot; and SHALL let the owner return a group to automatic. A group's name SHALL always be shown with its colour.

#### Scenario: New groups
- **WHEN** the owner creates two groups one after the other and picks no colour
- **THEN** they get the first two palette colours in order, and keep them when the groups are reordered or renamed

#### Scenario: Picking a colour
- **WHEN** the owner clicks a group's dot and chooses "Green"
- **THEN** the dot and the line along the group's rows turn green, and stay green after the app and the host restart

#### Scenario: Back to automatic
- **WHEN** the owner chooses "Automatic" in that popover
- **THEN** the group shows its automatic colour again

#### Scenario: Collapsed group
- **WHEN** a coloured group is collapsed
- **THEN** its header keeps the dot, and its chip in the icon rail is outlined in its colour

### Requirement: Removing an inactive session
The system SHALL remove an inactive session from the sidebar with the ✕ on its row after the owner confirms "Remove “title” from the sidebar?", keeping it in the index so Add sessions can offer it again.

#### Scenario: Removed
- **WHEN** the owner presses the ✕ of an inactive session and confirms
- **THEN** the session leaves the sidebar and Add sessions offers it again

#### Scenario: Remove cancelled
- **WHEN** the owner presses the ✕ of an inactive session and cancels
- **THEN** the session stays in the sidebar, inactive
