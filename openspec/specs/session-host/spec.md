# session-host Specification

## Purpose
Keeps every Headquarter agent session in one long-lived place the owner can start, list, attach to from any client, and resume, independent of any window.

## Requirements

### Requirement: Sessions outlive clients
The system SHALL keep each agent session it hosts running when the client that started or attached to it disconnects, until the agent exits or the owner ends it, and SHALL then have the agent redraw at the size of the client in use among those still attached. A client that attaches SHALL receive the terminal modes the agent has in effect before its recent output, however long ago the agent set them.

#### Scenario: Terminal closed while attached
- **WHEN** the owner closes the terminal that is attached to a working session
- **THEN** the agent keeps running and its status continues to update

#### Scenario: Attach again later
- **WHEN** the owner attaches to that session from another terminal
- **THEN** the session's recent output is shown and the agent redraws its interface at the new terminal's size

#### Scenario: Second client leaves
- **WHEN** the app shows a session at 120 columns, `hq attach` joins at 110 columns and then detaches
- **THEN** the agent redraws at 120 columns and the app's view is drawn correctly

#### Scenario: Modes set long before attaching
- **WHEN** a full-screen agent set the alternate screen, mouse reporting and bracketed paste at startup and has since printed more output than the host keeps, and the app attaches
- **THEN** the app's terminal is in those modes, so drags, clicks and the wheel go to the agent and pastes are bracketed

### Requirement: Native agent interface
The system SHALL present an attached session with the agent's own terminal interface and pass every key the owner types to the agent, except the documented detach key.

#### Scenario: Option question answered through attach
- **WHEN** an attached agent shows a multiple-choice question and the owner selects an option with arrow keys and Enter
- **THEN** the agent receives the selection exactly as in a plain terminal

### Requirement: Start on demand, single instance
The system SHALL start its host process automatically when a client needs it and SHALL refuse to run a second host process for the same user, waiting instead for a host that is stopping to exit.

#### Scenario: First command starts the host
- **WHEN** the owner runs a session command and no host process is running
- **THEN** the host starts and the command completes without a separate start step

#### Scenario: Second host refused
- **WHEN** a host process is already running and another is started
- **THEN** the second exits with an "already running" message and the first is unaffected

#### Scenario: Started while another stops
- **WHEN** a host is stopping (its socket removed, its agents still ending) and a client starts a new one
- **THEN** the new host waits for the stopping one to exit, then runs, and the client connects to it

### Requirement: Unique session names
The system SHALL keep session names unique across all sessions, so a name always identifies one session.

#### Scenario: Name already used
- **WHEN** the owner starts or renames a session with a name another session has
- **THEN** the request fails saying the name is already used and nothing changes

### Requirement: Persistent index
The system SHALL persist each session's metadata (agent, folder, name, agent conversation id, status, timestamps) across host restarts, SHALL mark sessions that were running when the host stopped without ending them as interrupted, and SHALL NOT persist terminal output, prompts, or model output.

#### Scenario: Host restarts
- **WHEN** the host process stops while sessions are live and is started again
- **THEN** those sessions are listed as idle and interrupted, with their names and folders intact

#### Scenario: Graceful stop
- **WHEN** the owner runs `hq stop`, confirms, and starts the host again
- **THEN** the sessions are listed as idle and none is interrupted

#### Scenario: Index contents
- **WHEN** the index is inspected after a session in which the owner typed prompts
- **THEN** it contains no prompt text and no terminal output

### Requirement: Resume
The system SHALL reopen an inactive session's conversation inside the host using the agent's own resume mechanism. When the conversation id is unknown, it SHALL open the agent's own conversation picker in the session's folder, or a new conversation there for an agent that has no picker. A Claude Code conversation that was never saved (no message sent) SHALL start afresh under the same id.

#### Scenario: Resume a known Claude conversation
- **WHEN** the owner resumes an inactive Claude session whose conversation id is recorded
- **THEN** a hosted session starts in the same folder continuing that conversation

#### Scenario: Conversation id unknown
- **WHEN** the owner resumes an inactive Claude or Codex session without a recorded conversation id
- **THEN** the agent starts in that folder showing its own conversation picker

#### Scenario: Agent without a picker
- **WHEN** the owner resumes an inactive agy session without a recorded conversation id
- **THEN** agy starts a new conversation in that folder

#### Scenario: Never used
- **WHEN** the owner resumes, or the host starts, a Claude session in which no message was ever sent
- **THEN** Claude Code starts a new conversation under the session's id, and no failure is reported

### Requirement: Clear failures
The system SHALL reject a new-session request with a specific reason when the agent cannot be started, and SHALL NOT record a session for it.

#### Scenario: Agent not installed
- **WHEN** the owner starts a session for an agent whose executable is not found
- **THEN** the request fails naming the missing executable and no session is listed

#### Scenario: Folder missing
- **WHEN** the owner starts a session in a folder that does not exist
- **THEN** the request fails naming the folder and no session is listed

### Requirement: Local-only access
The system SHALL accept client connections only from the owner's user account on the same machine.

#### Scenario: Socket permissions
- **WHEN** the host is running
- **THEN** its control and hook sockets are readable and writable by the owner only

### Requirement: Ending sessions
The system SHALL close a session on request by removing it from the sidebar at once and then, in the background, stopping its agent through the agent's own interrupt and exit commands, stopping the work the agent left running for it, and falling back to hanging up and then killing the agent when it does not exit. It SHALL keep a session whose agent ends without being closed in the sidebar, as inactive.

#### Scenario: Close an idle Claude session
- **WHEN** the owner closes an idle Claude Code session
- **THEN** Headquarter types `/exit`, the agent exits with code 0, and the session leaves the sidebar

#### Scenario: Close a Codex session with background work
- **WHEN** the owner closes a Codex session that has a background shell command running
- **THEN** Headquarter types `/stop` and `/quit`, the background command ends, and the session leaves the sidebar

#### Scenario: Agent ignores its exit command
- **WHEN** the agent has not exited 10 seconds after its exit command
- **THEN** the session has already left the sidebar, and the agent is hung up, then killed 3 seconds later if still running

#### Scenario: Quit from inside the agent
- **WHEN** the owner types `/exit` in a Claude Code session
- **THEN** the session stays in the sidebar as inactive

#### Scenario: Outside session
- **WHEN** the owner closes a session marked "ext"
- **THEN** it leaves the sidebar and Headquarter does not signal the other process

### Requirement: Stopping the host
The system SHALL end all sessions and itself only after the owner confirms, removing none of them from the sidebar and marking each to start again when the host next starts, and SHALL also stop a host left running by another Headquarter version.

#### Scenario: Stop everything
- **WHEN** the owner asks to stop the host while sessions are live and confirms
- **THEN** every hosted agent is ended, the sessions stay in the sidebar marked to start with the next host, and the host exits

#### Scenario: Stop declined
- **WHEN** the owner asks to stop the host and declines the confirmation
- **THEN** nothing is ended

#### Scenario: Host left by another version
- **WHEN** a host started by an earlier Headquarter version is running and the owner runs `hq stop --yes` from the new version
- **THEN** that host and its sessions end, and the command names the host's protocol version

#### Scenario: Another version, not confirmed
- **WHEN** a host from another version is running and the owner runs `hq stop` without `--yes` and without a terminal to answer in
- **THEN** nothing is stopped and the command says to pass `--yes`

### Requirement: No resume while open elsewhere
The system SHALL refuse to resume a conversation whose agent is running in another terminal, naming that process, and SHALL NOT signal or end the other process.

#### Scenario: Conversation open in another terminal
- **WHEN** the owner resumes a conversation that is open in another terminal
- **THEN** the request fails naming the other process id and asking the owner to close it there first, and the other process is unaffected

#### Scenario: Outside session closed
- **WHEN** the owner closes the agent in the other terminal and then resumes the conversation
- **THEN** a hosted session starts continuing that conversation

### Requirement: Agents found independent of the host PATH
The system SHALL locate the agent to start through agent discovery and the settings file, so a host started without the owner's shell PATH can start every installed hostable agent.

#### Scenario: Host started from a desktop launcher
- **WHEN** the host runs with a PATH of only system folders and Claude Code is installed in the owner's local bin folder
- **THEN** starting a Claude session succeeds using that installation

#### Scenario: Agent missing everywhere
- **WHEN** the owner starts a session for an agent found in no searched location
- **THEN** the request fails naming the agent and listing the places searched

### Requirement: Session list by the owner's groups
The system SHALL list the sidebar's sessions as the app shows them: "Needs you" while any session needs the owner, then each group in the owner's order with its count, then the ungrouped sessions, each line with agent, name, folder, status and age since last activity. It SHALL list every session it knows on request.

#### Scenario: Two groups
- **WHEN** the owner has groups "Backend" and "Docs" with one session each and one ungrouped session
- **THEN** `hq ls` shows "Backend", "Docs" and "Ungrouped" in that order, each with its session

#### Scenario: Same folder name in different places
- **WHEN** two listed sessions' folders have the same final name
- **THEN** their folder labels include enough of the parent path to tell them apart

#### Scenario: Every session requested
- **WHEN** the owner lists every session
- **THEN** sessions outside the sidebar are listed too, those in the sidebar are marked, and each shows its group if it has one

### Requirement: Last activity
The system SHALL date a session by the agent's last change of status. Reading a finished session, a session going idle, an agent stopping, and an agent starting or resuming before the owner has typed (Codex loading a conversation, agy signing in) SHALL NOT count as activity. Ages under a minute SHALL read "0m".

#### Scenario: Reading a finished session
- **WHEN** a session finished ten hours ago and the owner opens it now
- **THEN** its status becomes done and its age still reads ten hours

#### Scenario: Opening a stopped Codex session
- **WHEN** the owner opens a Codex session last active 40 minutes ago, and Codex reports a short turn while it loads the conversation
- **THEN** the row still reads 40m and keeps its place in the list

### Requirement: Groups kept by the host
The system SHALL keep the owner's groups, their order, their collapsed state and each session's group across host and app restarts, and SHALL tell every connected client when they change.

#### Scenario: Restart
- **WHEN** the host restarts
- **THEN** the same groups, in the same order, hold the same sessions

#### Scenario: Two clients
- **WHEN** the owner moves a session into a group in the app while `hq ls` is run in a terminal
- **THEN** `hq ls` run after the move shows the session in that group

### Requirement: Agents find their runtime
The system SHALL start every agent process with a `PATH` that holds the agent's own folder first, then the owner's login-shell `PATH` once known, then the host's own, and SHALL NOT read usage before the login-shell `PATH` is known, unavailable, or turned off.

#### Scenario: Node-based CLI from a desktop launch
- **WHEN** the app is opened from Finder and Codex is a Node script installed by Homebrew
- **THEN** Codex's sessions, version and usage work, because `node` is found in Codex's own folder

#### Scenario: Usage at start
- **WHEN** the host has just started and the login-shell `PATH` is not yet known
- **THEN** no usage read starts until it is known or the capture has finished

### Requirement: The first list holds past conversations
The system SHALL answer the first list request after the host starts only once its first import of the agents' history has finished, waiting at most 10 seconds.

#### Scenario: Opened right after the host starts
- **WHEN** a client lists sessions while the host's first history import is still running
- **THEN** the answer comes once the import has finished and includes the imported conversations

### Requirement: Sessions start with the host
The system SHALL, when the host starts, start again on its conversation every sidebar session that was running when the host last stopped, whatever stopped it, one at a time in sidebar order, starting a session a client asks for ahead of its turn, SHALL leave inactive sessions and conversations open in another terminal alone, and SHALL make a session that cannot start inactive with the reason.

#### Scenario: After a reboot
- **WHEN** three sidebar sessions were running, one was inactive, and the machine restarted and the owner opens Headquarter
- **THEN** the three start one after another without the owner opening them, and the inactive one stays inactive

#### Scenario: After hq stop
- **WHEN** the owner ran `hq stop --yes` with two sessions running and later opens Headquarter
- **THEN** both sessions start again

#### Scenario: Paced
- **WHEN** several sessions start with the host
- **THEN** each starts after the previous one has first reported its status, or 5 seconds after it started

#### Scenario: Shown session first
- **WHEN** the app opens showing a session that is still waiting its turn
- **THEN** that session starts at once, before the others

#### Scenario: Cannot start
- **WHEN** a session's folder was deleted while the host was stopped
- **THEN** the session becomes inactive, keeps the reason, and the other sessions still start

#### Scenario: Open in another terminal
- **WHEN** a session's conversation is running in another terminal when the host starts
- **THEN** the host does not start it

#### Scenario: Upgrade
- **WHEN** the first host of this version starts after an older host was stopped with two sessions running and one session already stopped
- **THEN** the two start, and the one already stopped is inactive

### Requirement: Terminal size follows the device in use
The system SHALL size a session's terminal for the client in use: the client that most recently attached to it, typed into it, or brought it to the front. A resize from another attached client SHALL be kept for when that client is in use. When the client in use leaves, the most recently used client still attached SHALL take over.

#### Scenario: Typing on the phone
- **WHEN** the desktop shows a session at 160 columns and a paired phone at 54 columns types into it
- **THEN** the agent redraws at 54 columns, on both

#### Scenario: Back on the desktop
- **WHEN** the owner then brings that session to the front on the desktop, or clicks in its terminal
- **THEN** the agent redraws at the desktop's 160 columns

#### Scenario: Another client resizes in the background
- **WHEN** the phone is in use and the desktop window is resized
- **THEN** the terminal keeps the phone's size, and the desktop's new size applies once the desktop is in use

### Requirement: Redraws the agent notices
When a viewer attaches to a session or takes it over, the system SHALL change the agent's terminal size in two steps the agent sees separately, one row fewer and then the viewer's size, restoring the size once the agent has repainted at the smaller size (it erases the screen) or after 500 ms, so the agent repaints at the viewer's size.

#### Scenario: Attach at the agent's current size
- **WHEN** the app attaches to a session at the size the agent already has
- **THEN** the agent sees one row fewer and then its size again, and repaints at that size

#### Scenario: Two attaches in quick succession
- **WHEN** two viewers attach to a session within 100 ms
- **THEN** the agent ends at the size of the viewer that attached last

#### Scenario: Agent writes nothing after a resize
- **WHEN** a viewer attaches to a shell that prints nothing after a size change
- **THEN** the size is restored after 500 ms

### Requirement: Claude sessions follow their conversation
The host SHALL keep each Claude session on the newest conversation Claude Code continues it in, recording a continuation as soon as it appears, so a resume after any restart continues where the owner left off.

#### Scenario: Continued under a new id
- **WHEN** Claude Code carries a hosted session's conversation on under a new id and the host restarts
- **THEN** the session resumes the new id, not the old one

### Requirement: Conversations running in Claude Code's background
The host SHALL NOT start an agent to resume a Claude conversation that Claude Code reports running in its background, and SHALL answer that the conversation is running there.

#### Scenario: Resume while running in the background
- **WHEN** the owner resumes a session whose conversation Claude Code runs in its background
- **THEN** no agent starts and the answer says it runs in Claude Code's background, with its process id

### Requirement: Merges keep the owner's rows
The host SHALL NOT delete a session that is in the sidebar or named by the owner when it merges two records of one conversation, and SHALL log every change to sidebar membership with the session ids.

#### Scenario: A named sidebar row and an imported copy
- **WHEN** a named sidebar session and an imported history row come to hold the same conversation
- **THEN** the named sidebar session stays, and the imported row is removed

### Requirement: Resources of running sessions
The host SHALL report, when a client asks, its own CPU and memory and, for each running session, the CPU and memory of the session's agent and all its descendant processes, and of its shell panel, with the number of processes beyond the agent. CPU SHALL be given per core and memory as resident memory.

#### Scenario: A session with a child process
- **WHEN** a session's agent has started a child process that uses CPU, and a client asks twice
- **THEN** the second answer includes the child in the session's figures and counts one process beyond the agent

### Requirement: Viewers told who is in use
The system SHALL tell each client attached to a session, or to its shell, which client is in use: itself, a client on the computer, or a paired phone by its name. It SHALL tell a client when it attaches, and SHALL tell every attached client whenever the client in use changes.

#### Scenario: The phone opens a session open on the desktop
- **WHEN** the desktop shows a session and the phone opens it
- **THEN** the phone is told it is in use, and the desktop is told the phone is in use, with the phone's name

#### Scenario: The desktop takes it back
- **WHEN** the owner then clicks in the desktop's terminal
- **THEN** the desktop is told it is in use, and the phone is told a client on the computer is in use

#### Scenario: The client in use leaves
- **WHEN** the phone in use closes the session and the desktop still shows it
- **THEN** the desktop is told it is in use

### Requirement: The session list names the phone in use
The system SHALL include, with each session it lists or announces, the name of the paired phone that is the client in use for it, or none, and SHALL announce the session again whenever that changes.

#### Scenario: A phone takes a session
- **WHEN** the phone "Pixel 8" opens a session the desktop also shows
- **THEN** subscribers receive the session again naming "Pixel 8"

#### Scenario: The phone leaves
- **WHEN** the phone closes the session
- **THEN** subscribers receive the session again naming no phone
