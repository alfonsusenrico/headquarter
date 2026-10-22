# session-host Specification

## Purpose
Keeps every Headquarter agent session in one long-lived place the owner can start, list, attach to from any client, and resume, independent of any window.

## Requirements

### Requirement: Sessions outlive clients
The system SHALL keep each agent session it hosts running when the client that started or attached to it disconnects, until the agent exits or the owner ends it, and SHALL then have the agent redraw at the size of the latest client still attached.

#### Scenario: Terminal closed while attached
- **WHEN** the owner closes the terminal that is attached to a working session
- **THEN** the agent keeps running and its status continues to update

#### Scenario: Attach again later
- **WHEN** the owner attaches to that session from another terminal
- **THEN** the session's recent output is shown and the agent redraws its interface at the new terminal's size

#### Scenario: Second client leaves
- **WHEN** the app shows a session at 120 columns, `hq attach` joins at 110 columns and then detaches
- **THEN** the agent redraws at 120 columns and the app's view is drawn correctly

### Requirement: Native agent interface
The system SHALL present an attached session with the agent's own terminal interface and pass every key the owner types to the agent, except the documented detach key.

#### Scenario: Option question answered through attach
- **WHEN** an attached agent shows a multiple-choice question and the owner selects an option with arrow keys and Enter
- **THEN** the agent receives the selection exactly as in a plain terminal

### Requirement: Start on demand, single instance
The system SHALL start its host process automatically when a client needs it and SHALL refuse to run a second host process for the same user.

#### Scenario: First command starts the host
- **WHEN** the owner runs a session command and no host process is running
- **THEN** the host starts and the command completes without a separate start step

#### Scenario: Second host refused
- **WHEN** a host process is already running and another is started
- **THEN** the second exits with an "already running" message and the first is unaffected

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
The system SHALL reopen an inactive session's conversation inside the host using the agent's own resume mechanism. When the conversation id is unknown, it SHALL open the agent's own conversation picker in the session's folder, or a new conversation there for an agent that has no picker.

#### Scenario: Resume a known Claude conversation
- **WHEN** the owner resumes an inactive Claude session whose conversation id is recorded
- **THEN** a hosted session starts in the same folder continuing that conversation

#### Scenario: Conversation id unknown
- **WHEN** the owner resumes an inactive Claude or Codex session without a recorded conversation id
- **THEN** the agent starts in that folder showing its own conversation picker

#### Scenario: Agent without a picker
- **WHEN** the owner resumes an inactive agy session without a recorded conversation id
- **THEN** agy starts a new conversation in that folder

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
The system SHALL close a session on request by removing it from the sidebar at once and then, in the background, stopping its agent through the agent's own interrupt and exit commands, stopping the work the agent left running for it, and falling back to hanging up and then killing the agent when it does not exit. It SHALL remove a session whose agent exits with code 0 from the sidebar and keep any other ended session in the sidebar.

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
- **THEN** the session leaves the sidebar

#### Scenario: Outside session
- **WHEN** the owner closes a session marked "ext"
- **THEN** it leaves the sidebar and Headquarter does not signal the other process

### Requirement: Stopping the host
The system SHALL end all sessions and itself only after the owner confirms, removing none of them from the sidebar, and SHALL also stop a host left running by another Headquarter version.

#### Scenario: Stop everything
- **WHEN** the owner asks to stop the host while sessions are live and confirms
- **THEN** every hosted agent is ended, the sessions stay in the sidebar, and the host exits

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
