# Spec Delta

## ADDED Requirements

### Requirement: Agents find their runtime
The system SHALL start every agent process with a `PATH` that holds the agent's own folder first, then the owner's login-shell `PATH` once known, then the host's own, and SHALL NOT read usage before the login-shell `PATH` is known, unavailable, or turned off.

#### Scenario: Node-based CLI from a desktop launch
- **WHEN** the app is opened from Finder and Codex is a Node script installed by Homebrew
- **THEN** Codex's sessions, version and usage work, because `node` is found in Codex's own folder

#### Scenario: Usage at start
- **WHEN** the host has just started and the login-shell `PATH` is not yet known
- **THEN** no usage read starts until it is known or the capture has finished
