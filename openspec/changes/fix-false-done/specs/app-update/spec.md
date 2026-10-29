## MODIFIED Requirements

### Requirement: Busy sessions before a restart
The system SHALL, before restarting for an update while any live sidebar session is working, needs the owner, shows background work, or has a plan to review, name those sessions and offer Restart Now or Wait, and with Wait SHALL install at once and restart by itself as soon as no session is busy.

#### Scenario: Ask first
- **WHEN** the owner presses Update Now while "Fix login" is working
- **THEN** the window names "Fix login", says restarting interrupts it, and offers Restart Now and Wait

#### Scenario: Wait
- **WHEN** the owner chooses Wait and "Fix login" finishes its turn
- **THEN** the new version is installed and the restart runs without another click

#### Scenario: Background work holds the restart
- **WHEN** the owner chooses Wait and "Fix login" finishes its turn while a background agent it started still runs
- **THEN** the restart waits until that agent ends

#### Scenario: A plan to review holds the restart
- **WHEN** the owner chooses Wait while an agy session has a plan to review
- **THEN** the restart waits until the owner approves or rejects it
