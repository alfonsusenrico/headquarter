## MODIFIED Requirements

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

## ADDED Requirements

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
