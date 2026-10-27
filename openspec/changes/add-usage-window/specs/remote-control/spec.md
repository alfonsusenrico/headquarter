## ADDED Requirements

### Requirement: Resources stay on the desktop
The host SHALL refuse a paired phone's request for the resources of its sessions.

#### Scenario: A phone asks for resources
- **WHEN** a paired phone sends the resources request
- **THEN** the host refuses it as it refuses any request outside the allow-list
