## ADDED Requirements

### Requirement: Host as a service on host-only installs
A host-only install SHALL run the host as the owner's user service, restarted when it exits, as a systemd user unit on Linux and a LaunchAgent on macOS, and SHALL say how to start the host when the system has neither.

#### Scenario: Server reboot
- **WHEN** the owner's Linux server restarts with lingering enabled
- **THEN** the host starts without anyone logging in, and the sessions that were running resume

#### Scenario: Lingering refused
- **WHEN** the installer cannot enable lingering
- **THEN** it prints `sudo loginctl enable-linger <user>` for the owner and installs everything else

#### Scenario: Update on the server
- **WHEN** the owner reruns the host-only install line and then `hq stop`
- **THEN** the service starts the new host and the sessions resume
