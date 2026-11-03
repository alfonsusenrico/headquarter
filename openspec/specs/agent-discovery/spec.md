# agent-discovery Specification

## Purpose
Lets Headquarter work on any macOS or Linux machine by detecting the platform and finding the coding-agent CLIs installed in their usual places, whether or not the current process has the owner's shell PATH.

## Requirements

### Requirement: Platform detection
The system SHALL report the operating system, CPU architecture, and, where they apply, the Linux distribution family, whether it runs under WSL, and the Homebrew prefix.

#### Scenario: Apple Silicon Mac
- **WHEN** discovery runs on macOS on an arm64 machine with Homebrew installed
- **THEN** it reports macOS, arm64, and the Homebrew prefix `/opt/homebrew`

#### Scenario: Ubuntu under WSL
- **WHEN** discovery runs on Linux whose os-release names Ubuntu and whose kernel identifies as Microsoft WSL
- **THEN** it reports Linux, the debian family, and WSL

### Requirement: Find installed agents
The system SHALL find each known agent CLI on the process PATH, the owner's login-shell PATH, and the known install folders for the detected platform, and SHALL report every location found and which one is used.

#### Scenario: Agent only in its own installer folder
- **WHEN** an agent is installed only in its own installer folder that is not on the process PATH
- **THEN** discovery finds it there and reports that folder

#### Scenario: Process started without the shell PATH
- **WHEN** the process PATH holds only system folders and the owner's login-shell PATH includes the agent's folder
- **THEN** discovery finds the agent through the login-shell PATH

#### Scenario: Two installs
- **WHEN** the same agent is installed in two folders
- **THEN** both are reported and the one that would be used is marked

#### Scenario: Not installed
- **WHEN** an agent is in no searched location
- **THEN** it is reported as not installed with the places searched

### Requirement: Login shell is bounded
The system SHALL stop waiting for the owner's login shell after a fixed timeout and continue with the other sources.

#### Scenario: Slow shell startup
- **WHEN** the login shell does not answer within the timeout
- **THEN** discovery completes using the process PATH and known folders and reports the login-shell PATH as unavailable

### Requirement: Versions and support
The system SHALL report each found agent's version when it answers within a fixed timeout, and whether Headquarter can host that agent.

#### Scenario: Supported and reported-only agents
- **WHEN** Claude Code and Gemini CLI are both installed
- **THEN** Claude Code is reported as hostable and Gemini CLI as found but not yet supported

#### Scenario: Version command hangs
- **WHEN** an agent's version command does not finish within the timeout
- **THEN** that agent is reported with version unknown and the other agents are unaffected

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
