# external-sessions Specification

## Purpose
Lets the owner see agent sessions they started in other terminals, with their folder and best-known status, without Headquarter ever interfering with those sessions.

## Requirements

### Requirement: Discover outside sessions
The system SHALL list agent sessions running outside it for each enabled agent, with the agent, working folder, process id, start time when known, and an external marker.

#### Scenario: Claude session in another terminal
- **WHEN** the owner has a Claude Code session running in another terminal
- **THEN** the scan lists it with its folder, session id, and status as reported by Claude Code

#### Scenario: Codex session in another terminal
- **WHEN** the owner has a Codex session running in another terminal
- **THEN** the scan lists it with its folder and process id

### Requirement: Best-known status for outside sessions
The system SHALL give each outside session the most specific status its sources support, and SHALL mark the status as unknown rather than guess when no source supports one.

#### Scenario: Status not derivable
- **WHEN** an outside session's agent provides no status source and its session data cannot be matched
- **THEN** the session is listed with status unknown

### Requirement: Observe-only
The system SHALL NOT signal, attach to, write to, or change the configuration of any outside session or its agent, and SHALL read only metadata (event types, timestamps, paths) from their session files.

#### Scenario: Scan runs while sessions are active
- **WHEN** the scan runs while outside sessions are active
- **THEN** those sessions continue unaffected and no file belonging to them or their agents is modified

#### Scenario: Session file contains conversation text
- **WHEN** the scan reads an outside session's file to derive status
- **THEN** no conversation text is stored or printed

### Requirement: Scan tolerates failure
The system SHALL continue the scan when one agent's source fails and SHALL report that source's failure.

#### Scenario: Claude CLI missing
- **WHEN** the Claude Code CLI is not installed or its session listing fails
- **THEN** the scan still lists other agents' sessions and reports that Claude's listing failed

### Requirement: Claude Code sessions from its records
The system SHALL read Claude Code's running sessions from Claude Code's own session records on every scan, SHALL start `claude agents --json` only once a minute or when a background agent's state file changed since its last run, and SHALL fall back to the command on every scan when Claude Code keeps no records.

#### Scenario: Records present
- **WHEN** Claude Code keeps session records and no background agent changed in the last minute
- **THEN** a scan lists Claude Code's running sessions with their status without starting any Claude Code process

#### Scenario: Ended session
- **WHEN** a record names a process that is no longer running Claude Code
- **THEN** the session is not listed

#### Scenario: Background agent changes
- **WHEN** a background agent's state file changes
- **THEN** the next scan starts `claude agents --json` and lists the background agents it reports, without reading the state file's contents

#### Scenario: Older Claude Code
- **WHEN** Claude Code keeps no session records
- **THEN** every scan starts `claude agents --json`, as before

### Requirement: Scan pace follows the owner's view
The system SHALL scan for outside sessions every 5 seconds while a client shows the sessions or a hosted session is working or has background work, SHALL scan once a minute otherwise, and SHALL scan at once when a client starts showing the sessions or lists them and the last scan is older than 5 seconds, without making a list request wait for that scan.

#### Scenario: Window hidden and nothing working
- **WHEN** the app's window is covered, minimized or hidden, no other client shows the sessions, and no hosted session is working
- **THEN** the host scans for outside sessions once a minute

#### Scenario: Window shown again
- **WHEN** the window becomes visible after a minute hidden
- **THEN** outside sessions are scanned at once and every 5 seconds after

#### Scenario: Hosted work while hidden
- **WHEN** the window is hidden and a hosted session is working
- **THEN** the host keeps scanning every 5 seconds

#### Scenario: Listing while nobody looks
- **WHEN** no client shows the sessions and `hq ls` asks for the list
- **THEN** the host answers at once from its last scan
- **AND** scans at once if that scan is older than 5 seconds
