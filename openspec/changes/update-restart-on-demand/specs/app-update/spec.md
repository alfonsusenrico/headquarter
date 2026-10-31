## MODIFIED Requirements

### Requirement: Busy sessions before a restart
The system SHALL, before restarting for an update while any live sidebar session is working, needs the owner, shows background work, or has a plan to review, name those sessions and offer to restart now or wait. Wait SHALL install at once and never restart by itself; the sidebar SHALL then offer "Update Now", which restarts at once when nothing is busy and otherwise asks again.

#### Scenario: Ask first
- **WHEN** the owner presses Update Now while "Fix login" is working
- **THEN** the window names "Fix login", says restarting interrupts it, and offers Restart Now and Wait

#### Scenario: Wait
- **WHEN** the owner chooses Wait and "Fix login" finishes its turn
- **THEN** the new version is installed, nothing restarts, and the sidebar shows "Update Now"

#### Scenario: Background work holds the restart
- **WHEN** the owner presses the sidebar's Update Now while a session's background agent still runs
- **THEN** the window names that session and offers Later and Restart Now

#### Scenario: A plan to review holds the restart
- **WHEN** the owner presses the sidebar's Update Now while an agy session has a plan to review
- **THEN** the window names that session and offers Later and Restart Now

#### Scenario: Nothing busy
- **WHEN** the update is installed, no session is busy, and the owner presses the sidebar's Update Now
- **THEN** Headquarter restarts onto the new version at once
