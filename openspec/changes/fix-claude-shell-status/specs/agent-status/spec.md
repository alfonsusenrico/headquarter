# Spec Delta

## MODIFIED Requirements

### Requirement: Background work counts as working
The system SHALL show a session as working while its agent runs work for it in the background, including Claude Code background shell commands and Codex sub-agents, even after the agent reports its turn finished.

#### Scenario: Claude background command
- **WHEN** a Claude Code session starts a 40-second shell command in the background and ends its turn
- **THEN** the session shows working until `claude agents --json` stops reporting it busy, then idle not read yet after Claude's follow-up turn

#### Scenario: Codex sub-agent
- **WHEN** a Codex session spawns a sub-agent and ends its turn while the sub-agent runs
- **THEN** the session shows working until the sub-agent's turn ends

#### Scenario: Background state unavailable
- **WHEN** `claude agents --json` fails
- **THEN** the Claude session's status follows its own reports alone

#### Scenario: Claude Code records "shell"
- **WHEN** a hosted Claude Code session ends its turn while its background shell command runs, and its session record says "shell"
- **THEN** the session shows working until the record changes, without waiting for `claude agents --json`
