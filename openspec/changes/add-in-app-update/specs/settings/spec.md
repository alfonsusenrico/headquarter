## ADDED Requirements

### Requirement: Update settings
The system SHALL check for updates automatically unless the settings file sets `[updates] check = false`, applied without a restart; Check for Updates… SHALL work either way.

#### Scenario: Checks turned off
- **WHEN** the settings file sets `[updates] check = false`
- **THEN** no automatic check runs, and Check for Updates… still checks
