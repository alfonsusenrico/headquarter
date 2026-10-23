# Spec Delta

## ADDED Requirements

### Requirement: agy usage from its own usage command
The system SHALL read agy's usage with agy's headless `/usage` command and its JSON output, run in an empty folder of Headquarter's own, SHALL report each window of each pool of models agy lists with the pool's name, and SHALL NOT change agy's settings files.

#### Scenario: agy reports two pools
- **WHEN** agy's `/usage` lists "Gemini Models" with a 5-hour window 42% used and "Claude and GPT models" with a weekly window 10% used
- **THEN** agy's details list "5h · Gemini" at 42% and "week · Claude and GPT" at 10%, with their reset times

#### Scenario: Nothing of the owner's is touched
- **WHEN** agy's usage is read
- **THEN** agy runs in Headquarter's `quota/agy-workspace` folder, no agy conversation is started, and no file under agy's settings is written by Headquarter

### Requirement: Rings for pools of models
The system SHALL fill the rings of an agent that reports only pools of models from the pool nearest its limit, by the highest whole percentage used and, on a tie, the shorter window; between equal pools, the first one the agent reports.

#### Scenario: Claude and GPT pool nearer its limit
- **WHEN** agy's Gemini pool has used 11% of its week and its Claude and GPT pool 85% of its five hours and 40% of its week
- **THEN** agy's inner ring is filled to 85% and its outer ring to 40%, both from the Claude and GPT pool, and the label reads "agy 85% · 5h · Claude and GPT"

## MODIFIED Requirements

### Requirement: Reading cadence
The system SHALL read Codex usage 10 seconds after a hosted Codex session's turn ends, every five minutes while a client shows usage, and on refresh; SHALL take Claude usage from each report of a hosted Claude session and with a hidden `/usage` read on refresh, every ten minutes while a client shows usage and no hosted Claude session has reported, and after a Claude session in another terminal finishes a turn, at most every two minutes; SHALL read agy usage 10 seconds after a hosted agy session's turn ends, every ten minutes while a client shows usage, and on refresh; and SHALL NOT read any of them when no client is showing usage.

#### Scenario: App closed
- **WHEN** no app window is open
- **THEN** no Codex reading, no hidden Claude read and no agy read is started

#### Scenario: Codex turn ends
- **WHEN** a hosted Codex session finishes a turn while the app is open
- **THEN** Codex usage is read again within about 10 seconds

#### Scenario: agy turn ends
- **WHEN** a hosted agy session finishes a turn while the app is open
- **THEN** agy usage is read again within about 10 seconds

#### Scenario: Read fails
- **WHEN** a Codex reading fails after an earlier one succeeded
- **THEN** the earlier numbers stay and the details say "Couldn't update since" the earlier reading's time

### Requirement: Unavailable quota is explicit
The system SHALL report a provider's quota as unavailable, with the reason, when it cannot be read, and SHALL NOT show estimated or stale values as current.

#### Scenario: No Claude session yet
- **WHEN** quotas are requested and no Claude reading has been captured
- **THEN** Claude's quota is reported as unavailable with the reason that no session has reported limits yet

#### Scenario: Provider exposes no quota source
- **WHEN** quotas are requested for a provider Headquarter has no way to read, such as agy when it cannot be found
- **THEN** that provider's quota is reported as "<Name> usage not available", for example "Agy usage not available"

#### Scenario: Old reading
- **WHEN** the latest reading for a provider is older than a window's reset time
- **THEN** that window is reported as stale with the reading time, not as current

### Requirement: Usage failures point at the provider
The system SHALL tell the owner, when a provider does not report its usage, which provider it was and what to try, without internal detail, SHALL record the reason in the host log, and SHALL try a failed Codex read again after a minute.

#### Scenario: Codex does not report
- **WHEN** Codex's usage read fails
- **THEN** the details read "Codex didn't report its usage. Open Codex once to check it is signed in, then refresh." and Codex is read again a minute later

#### Scenario: agy does not report
- **WHEN** agy's `/usage` read fails or times out
- **THEN** the details read "agy didn't report its usage. Open agy once to check it is signed in, then refresh."

#### Scenario: An agent Headquarter cannot find
- **WHEN** agy is enabled but Headquarter cannot find it
- **THEN** its usage details read "Agy usage not available", the same line any agent without a usage source gets, with no further detail

#### Scenario: Claude Code asks to trust a folder
- **WHEN** Claude Code's hidden usage read stops at its folder-trust question
- **THEN** the details say Claude Code asked to trust a folder and that opening Claude Code once and answering fixes it
