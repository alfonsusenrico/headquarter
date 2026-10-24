## MODIFIED Requirements

### Requirement: Start on demand, single instance
The system SHALL start its host process automatically when a client needs it and SHALL refuse to run a second host process for the same user, waiting instead for a host that is stopping to exit.

#### Scenario: First command starts the host
- **WHEN** the owner runs a session command and no host process is running
- **THEN** the host starts and the command completes without a separate start step

#### Scenario: Second host refused
- **WHEN** a host process is already running and another is started
- **THEN** the second exits with an "already running" message and the first is unaffected

#### Scenario: Started while another stops
- **WHEN** a host is stopping (its socket removed, its agents still ending) and a client starts a new one
- **THEN** the new host waits for the stopping one to exit, then runs, and the client connects to it
