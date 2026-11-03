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

#### Scenario: A check that cannot run
- **WHEN** Codex's most recent session folder asks whether to trust it
- **THEN** the status check is recorded as not checked, and no notice appears

#### Scenario: Once per version
- **WHEN** a failed check's notice was shown for Claude Code 2.1.316 and the host restarts
- **THEN** the version is not checked again and the notice does not return

#### Scenario: Checking on demand
- **WHEN** the owner runs `hq agents --check`
- **THEN** each agent turned on is checked now and its results are printed

#### Scenario: Not from another device
- **WHEN** a paired phone or computer asks the host to check its agents
- **THEN** the host refuses
