## ADDED Requirements

### Requirement: Checking an agent after it updates
When an installed agent's version changes, the system SHALL check, without sending a prompt, that the agent reports its status at start and that its usage reads, SHALL log each result, and SHALL tell the owner once per version, in a notice that stays until dismissed, which check failed.

#### Scenario: A usage format changes
- **WHEN** Claude Code updates to a version whose usage screen Headquarter cannot read
- **THEN** a notice says "Claude Code 2.1.316: Headquarter can't read its usage", and the host log records the failed check

#### Scenario: An update that changes nothing for Headquarter
- **WHEN** Codex updates and both checks pass
- **THEN** no notice appears, and the host log records the passing check

#### Scenario: No tokens spent
- **WHEN** a check runs
- **THEN** no prompt is sent to any agent
