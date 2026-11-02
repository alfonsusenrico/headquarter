## ADDED Requirements

### Requirement: Pairing a desktop
The system SHALL pair a desktop app with the same one-time code as a phone, record it as a desktop, and list it with its kind among the paired devices.

#### Scenario: Desktop pairs
- **WHEN** the owner shows a code on the host and enters it in Connect to a Host on another computer
- **THEN** the host records that computer as a paired desktop and the Remote Control window lists it as a desktop

#### Scenario: Older phone pairs
- **WHEN** a phone app that does not name its kind pairs
- **THEN** the host records it as a phone

### Requirement: What a desktop may do
The system SHALL let a paired desktop send every request the local app sends except stopping the host, showing pairing codes, and listing or removing devices, and SHALL refuse those, keeping the connection.

#### Scenario: Desktop starts a session
- **WHEN** a paired desktop asks the host to start a new Claude Code session in a folder
- **THEN** the session starts on the host and appears in every client's sidebar

#### Scenario: Desktop asks to stop the host
- **WHEN** a paired desktop sends a request to stop the host or to show a pairing code
- **THEN** the host refuses it, nothing changes, and the desktop stays connected

### Requirement: Several streams on one connection
The system SHALL serve up to 64 streams on one paired device's connection, each as its own host client with the same checks, and SHALL close them all when the device is removed.

#### Scenario: Terminals in parallel
- **WHEN** a paired desktop follows the sidebar and opens two terminals at once
- **THEN** all three streams work at the same time over one connection

#### Scenario: Device removed
- **WHEN** the owner removes a paired desktop with three open streams
- **THEN** all three streams close and the desktop says it was removed

### Requirement: Folders on the host
The system SHALL list a host folder's subfolders for the desktop app, leaving hidden folders out, and SHALL refuse the listing to phones.

#### Scenario: Browsing the server's folders
- **WHEN** the New session folder browser on a remote client opens the home folder
- **THEN** it shows the host's folders, not the client computer's

#### Scenario: Phone asks for a folder
- **WHEN** a paired phone asks for a folder listing
- **THEN** the host refuses it and the phone stays connected
