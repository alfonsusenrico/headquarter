## MODIFIED Requirements

### Requirement: Remote control settings
The system SHALL use n0's public relays for remote control unless `[remote] relays` lists the owner's own relay URLs, which it SHALL then use instead, and SHALL accept `[remote] enabled` in older settings files without acting on it.

#### Scenario: Default
- **WHEN** the settings file has no `[remote]` section and no phone is paired
- **THEN** remote control does not run and the host opens no network endpoint

#### Scenario: Own relay
- **WHEN** the settings file sets `[remote] relays = ["https://relay.example.com"]` and remote control runs
- **THEN** the host and its pairing codes use that relay and no other

#### Scenario: Invalid relay
- **WHEN** `[remote] relays` holds something that is not an https URL
- **THEN** the settings error names the file, the line and the problem, as for other settings
