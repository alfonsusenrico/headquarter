# remote-control Specification

## Purpose
Lets the owner follow and answer their sessions from a paired phone, anywhere, while nothing is reachable until they turn it on and a phone can do only what the host allows.

## Requirements

### Requirement: Remote control off until turned on
The system SHALL run no network endpoint for remote control until the owner turns it on, SHALL keep the host running while it is on, and SHALL stop it and close every phone connection when the owner turns it off.

#### Scenario: Turned on
- **WHEN** the owner runs `hq remote on`
- **THEN** the settings file gains `[remote] enabled = true` with its other content kept, and the host starts its gateway without a restart

#### Scenario: Kept up for phones
- **WHEN** remote control is on and no session runs and no client is connected
- **THEN** the host keeps running, since a phone cannot start it

#### Scenario: Turned off with a phone connected
- **WHEN** a phone is connected and the owner runs `hq remote off`
- **THEN** the phone's connection closes and the host opens no network endpoint

### Requirement: Pairing with a one-time code
The system SHALL pair a phone only with a code the owner has just shown, valid once and for 2 minutes, and SHALL record the paired phone's name, identity and pairing time.

#### Scenario: Pairing
- **WHEN** the owner runs `hq remote pair` and a phone presents the shown code within 2 minutes
- **THEN** the phone is paired, `hq remote pair` reports its name, and `hq remote devices` lists it

#### Scenario: Expired or reused code
- **WHEN** a phone presents a code older than 2 minutes, or one another phone already used
- **THEN** the connection is refused and nothing is recorded

#### Scenario: Cancelled
- **WHEN** the owner closes the window showing the code, or stops `hq remote pair`, before a phone uses it
- **THEN** the code stops working at once

#### Scenario: Guessing
- **WHEN** five wrong codes arrive within a minute
- **THEN** the code being shown stops working and the owner must show a new one

### Requirement: Only paired phones connect
The system SHALL accept a phone connection only from a paired phone or one presenting a valid pairing code, encrypted end to end and authenticated by the phone's key, and SHALL record when each paired phone was last seen.

#### Scenario: Unknown phone
- **WHEN** a phone that is not paired connects without a pairing code
- **THEN** the connection is closed before any session information is sent

#### Scenario: From another network
- **WHEN** a paired phone on mobile data connects while the computer is on Wi-Fi
- **THEN** it connects through a relay that cannot read the traffic, and `hq remote devices` shows it seen now

### Requirement: What a phone may do
The system SHALL let a paired phone list and follow sessions, open and type into them, resize, resume inactive sessions, add past conversations, open a session's shell, mark sessions seen and read usage, and SHALL refuse every other request, keeping the connection.

#### Scenario: Typing
- **WHEN** a paired phone attaches to a session and sends text
- **THEN** the agent receives it as typed

#### Scenario: Refused request
- **WHEN** a paired phone asks to start a new session, rename, close or remove one, edit a group, close a shell or stop the host
- **THEN** the host answers with a refusal, nothing changes, and the phone stays connected

### Requirement: Removing a phone
The system SHALL let the owner remove a paired phone by name or identity, closing its connection at once and refusing it afterwards until it pairs again.

#### Scenario: Lost phone
- **WHEN** the owner runs `hq remote remove "Pixel 8"` while that phone is connected
- **THEN** its connection closes, it disappears from `hq remote devices`, and its next connection is refused

### Requirement: Remote control logs without content
The system SHALL log remote control events (turned on or off, paired, connected with its path, disconnected, refused, failed pairing) without secrets, keys, typed input or terminal output.

#### Scenario: A phone types
- **WHEN** a paired phone types a prompt into a session
- **THEN** the host log records the connection but not the text

### Requirement: Remote Control window
The system SHALL offer, from the app's menu, a Remote Control window with a switch for remote control, how the computer is reached, a Pair a Phone button that shows the pairing code as an image QR code with its remaining time and as text, and the paired phones with whether each is connected or when it was last seen and a Remove button.

#### Scenario: Pairing from the window
- **WHEN** the owner opens Headquarter ▸ Remote Control…, presses Pair a Phone and scans the QR code with a phone
- **THEN** the window says the phone is paired and lists it as connected

#### Scenario: Turned on from the window
- **WHEN** remote control is off and the owner turns the switch on
- **THEN** the settings file gains `[remote] enabled = true`, the host starts its gateway, and the window says how the computer is reached

#### Scenario: Terminal too small for the QR code
- **WHEN** the owner runs `hq remote pair` in a terminal smaller than the QR code
- **THEN** the command names the size it needs and shows the code as text instead of a broken QR code

### Requirement: Welcome on every phone connection
The system SHALL send a paired phone, on each connection and before any answer from the host, the computer's name, its current relay, and the host's protocol and version.

#### Scenario: Relay changed
- **WHEN** the host's home relay changed since a phone paired, and the phone connects
- **THEN** the phone learns the new relay from the welcome and uses it next time

#### Scenario: Version check
- **WHEN** a phone connects to a host that speaks an older protocol
- **THEN** the phone receives the host's protocol and version before the host's answer to its greeting

### Requirement: A phone removes itself
The system SHALL let a paired phone remove its own pairing, deleting only that phone's record and closing its connection, and SHALL refuse the same request from a local client.

#### Scenario: Unpair from the phone
- **WHEN** a paired phone sends Unpair
- **THEN** the host answers, closes that connection, and the phone disappears from `hq remote devices` and the Remote Control window

#### Scenario: Local client
- **WHEN** a client on the local socket sends Unpair
- **THEN** the host refuses it as a bad request and removes nothing

### Requirement: Stopping the host closes phones
The system SHALL close every phone's connection when the host stops, so connected phones learn it at once instead of after the connection's idle timeout.

#### Scenario: `hq stop` with a phone connected
- **WHEN** the host stops while a phone is connected
- **THEN** the phone's connection closes within seconds, and the phone shows Reconnecting

### Requirement: A known phone pairs again
The system SHALL accept a shown pairing code from a phone it already knows, keeping one record for that phone under the name it sends.

#### Scenario: Scanning again
- **WHEN** a phone the computer still lists scans a new code from Remote Control
- **THEN** it pairs, and `hq remote devices` lists it once
