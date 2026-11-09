# remote-control Specification

## Purpose
Lets the owner follow and answer their sessions from a paired phone, anywhere, while nothing is reachable until they turn it on and a phone can do only what the host allows.

## Requirements

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
The system SHALL let a paired phone list and follow sessions, open and type into them, resize, resume inactive sessions, add past conversations, open a session's shell, mark sessions seen, read usage and send images to a session, and SHALL refuse every other request, keeping the connection.

#### Scenario: Typing
- **WHEN** a paired phone attaches to a session and sends text
- **THEN** the agent receives it as typed

#### Scenario: Refused request
- **WHEN** a paired phone asks to start a new session, rename, close or remove one, edit a group, close a shell or stop the host
- **THEN** the host answers with a refusal, nothing changes, and the phone stays connected

#### Scenario: Sending an image
- **WHEN** a paired phone uploads a screenshot to one of the host's sessions
- **THEN** the host saves it and answers with its path, and the phone stays connected

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
The system SHALL offer, from the app's menu, a Remote Control window with how the computer is reached while remote control runs, a Pair a Phone button that shows the pairing code as an image QR code with its remaining time and as text, and the paired phones with whether each is connected or when it was last seen and a Remove button. It SHALL have no switch for remote control.

#### Scenario: Pairing from the window
- **WHEN** the owner opens Headquarter ▸ Remote Control…, presses Pair a Phone and scans the QR code with a phone
- **THEN** the window says the phone is paired and lists it as connected

#### Scenario: Turned on from the window
- **WHEN** no phone is paired and the owner presses Pair a Phone
- **THEN** the host starts its endpoint for the code, and the window says how the computer is reached

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

### Requirement: Resources stay on the desktop
The host SHALL refuse a paired phone's request for the resources of its sessions.

#### Scenario: A phone asks for resources
- **WHEN** a paired phone sends the resources request
- **THEN** the host refuses it as it refuses any request outside the allow-list

### Requirement: Getting the phone app
The Remote Control window SHALL always show, as its first step, a QR code and a Copy link button for the newest stable Headquarter for Android APK, through an address that always names the latest release and skips pre-releases.

#### Scenario: Scanning for the app
- **WHEN** the owner opens Headquarter ▸ Remote Control… with no phone paired and scans the first QR code with the phone's camera
- **THEN** the phone downloads `headquarter-android.apk` from the latest stable release

#### Scenario: Copying the link
- **WHEN** the owner presses Copy link
- **THEN** the clipboard holds the latest-download address and the button reads "Copied" for a moment

#### Scenario: Remote control off
- **WHEN** no phone is paired, so remote control is not running
- **THEN** the step still shows, so the app can be installed before pairing

#### Scenario: A phone already paired
- **WHEN** at least one phone is paired
- **THEN** the QR code and Copy link still show in full, with no Show button

### Requirement: The host names the phone
The system SHALL name a paired phone's connection to the host with that phone's name from the index, and SHALL refuse a phone's own request to name its connection, keeping the connection.

#### Scenario: Named by the host
- **WHEN** a phone paired as "Pixel 8" opens a session that the desktop also shows
- **THEN** the desktop is told that "Pixel 8" is in use

#### Scenario: A phone names itself
- **WHEN** a phone sends a request naming its own connection
- **THEN** the request is refused, the connection stays open, and the name is unchanged

### Requirement: Remote control runs while a phone is paired
The system SHALL run the remote control endpoint only while at least one phone is paired or a pairing code is shown, SHALL keep the host running while it runs, and SHALL stop it and close every phone connection when the last phone is removed.

#### Scenario: No phone paired
- **WHEN** no phone is paired and no pairing code is shown
- **THEN** the host opens no network endpoint

#### Scenario: Pairing the first phone
- **WHEN** no phone is paired and the owner shows a pairing code
- **THEN** the host starts the endpoint, and after a phone pairs it keeps running

#### Scenario: A code expires unused
- **WHEN** no phone is paired and the shown code expires or is cancelled
- **THEN** the endpoint stops

#### Scenario: The last phone removed
- **WHEN** the owner removes the only paired phone while it is connected
- **THEN** its connection closes, the phone says it was removed from the computer, and the host opens no network endpoint

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

### Requirement: A device's own use of a session
The system SHALL name the paired device using a session to the host's other clients, and SHALL leave that name out of what it sends to the device itself.

#### Scenario: Desktop opens a server session
- **WHEN** a paired desktop opens a session on the server
- **THEN** the server's own app marks the session as in use on that desktop, and the desktop's sidebar shows no such mark for it
