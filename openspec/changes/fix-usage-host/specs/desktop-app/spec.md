## REMOVED Requirements

### Requirement: Usage follows the selected session
**Reason**: The footer flipped to another host's usage as the owner selected sessions, so one host's failed read looked like this computer's (owner, 2026-11-10).
**Migration**: The footer shows one host by the "Usage from one host" requirement. The Usage window keeps following the selected session by "Usage window follows the selected session".

## ADDED Requirements

### Requirement: Usage from one host
The usage footer SHALL show one host's usage and SHALL NOT change when the owner selects a session. The host SHALL be the one the owner chose in the usage details while the app holds its usage, else this computer when it runs a host, else the first paired host the app holds usage for. The footer SHALL name the host when it is not this computer. The app SHALL remember the choice between runs and drop it when that host is forgotten.

#### Scenario: Selecting a server session
- **WHEN** this Mac runs a host and the owner selects a session on the paired server
- **THEN** the footer keeps showing this Mac's usage, with no host name

#### Scenario: Choosing the server
- **WHEN** the owner opens the usage details and chooses the server under "Usage from"
- **THEN** the footer shows the server's rings named "server", and still does after the app is reopened

#### Scenario: A client without its own host
- **WHEN** the app runs on Windows with two paired servers and no choice made
- **THEN** the footer shows the first paired server's usage, named

#### Scenario: The chosen host is away
- **WHEN** the owner chose the server and the server stops answering
- **THEN** the footer shows this Mac's usage, and the server's when it lists its sessions again

#### Scenario: Forgetting the chosen host
- **WHEN** the owner chose the server and then forgets it
- **THEN** the footer shows this Mac's usage, and the app no longer keeps the choice

#### Scenario: One host
- **WHEN** only one host is known to the app
- **THEN** the usage details show no "Usage from" row

### Requirement: Usage window follows the selected session
The Usage window (CPU and memory) SHALL measure the selected session's host, else the first host, and SHALL name the host when it is not this computer.

#### Scenario: A server session selected
- **WHEN** the owner selects the server's health agent and opens the Usage window
- **THEN** the window measures the server and lists the server's sessions, named "host on server"
