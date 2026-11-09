## MODIFIED Requirements

### Requirement: Client of the host
The system SHALL connect as a client to this computer's host, starting it when needed, and to every paired host on another computer at once, and SHALL leave every session running when the window or the app closes.

#### Scenario: App closed mid-turn
- **WHEN** the owner quits the app while an agent is working
- **THEN** the agent keeps working and `hq ls` shows it

#### Scenario: Remote host on screen
- **WHEN** the owner has paired a server
- **THEN** the sidebar shows the server's sessions together with this computer's, and their terminals and usage come from the server

### Requirement: Reconnecting to a remote host
While a host is unreachable, the app SHALL keep that host's sessions in the list greyed with "Reconnecting…", retry by itself, and reattach its terminals when it returns, leaving the other hosts' sessions as they are.

#### Scenario: Network drop
- **WHEN** the network to the server drops for 20 seconds
- **THEN** the server's sessions show greyed with "Reconnecting…", this computer's sessions stay as they are, and when the network returns the server's sessions and an open terminal on the server come back without the owner acting

#### Scenario: Removed on the host
- **WHEN** the owner removes this computer on the host
- **THEN** the app stops retrying that host, its sessions leave the list, and the Hosts menu says the host no longer knows this computer and to pair it again with Connect to a Host

#### Scenario: Opening with the host away
- **WHEN** the app opens and a paired host cannot be reached yet
- **THEN** this computer's sessions show at once, and the Hosts menu says the host is connecting

### Requirement: A remote host is never stopped from the app
Quitting or updating the app SHALL act on this computer only: it SHALL never stop, or ask about stopping, a host on another computer.

#### Scenario: Quit with the server on screen
- **WHEN** the owner quits the app while the server's sessions are in the list
- **THEN** the app asks only about this computer's running sessions, and the server's sessions keep running

## REMOVED Requirements

### Requirement: Host switcher
**Reason**: One list shows every host's sessions; there is no host on screen to switch.
**Migration**: The Hosts menu (added here) lists the hosts with Connect to a Host and Forget.

## ADDED Requirements

### Requirement: Every host in one list
The sidebar SHALL list the sessions of this computer's host and of every paired host together. A session on another computer SHALL show that computer's name; on a computer without its own host, every session SHALL show its host's name. Every action on a session SHALL go to its host.

#### Scenario: The Mac and the server
- **WHEN** the owner on the Mac has a server paired with a health agent running there
- **THEN** the sidebar lists the Mac's sessions without a host name and the health agent with "server"

#### Scenario: On Windows
- **WHEN** the owner on a Windows PC has one server paired
- **THEN** every session shows "server"

#### Scenario: Renaming a server session
- **WHEN** the owner renames the server's health agent from the Mac's sidebar
- **THEN** the server renames it, and the phone paired with the server shows the new name

### Requirement: Groups across hosts
The app SHALL show groups of the same name on different hosts as one group. Moving a session into a group SHALL use that host's group of the name, making it there when missing. Renaming, recolouring, collapsing, moving and deleting a group SHALL apply on every host that has it.

#### Scenario: A server session joins the Mac's group
- **WHEN** the Mac has a group "ops" and the owner drags the server's health agent into it
- **THEN** the server gets a group "ops" holding the agent, and the sidebar shows one "ops" with the Mac's sessions and the agent

#### Scenario: Renaming a shared group
- **WHEN** "ops" exists on the Mac and on the server and the owner renames it to "infra"
- **THEN** both hosts rename their "ops" to "infra"

### Requirement: Hosts menu
The sidebar header SHALL open a Hosts menu listing this computer when it has a host and each paired host with its state (connected, reconnecting, not reachable), Connect to a Host, and Forget for each paired host. Forget SHALL ask in place before forgetting: the row SHALL read "Forget <host>? You'll need a new code from it to connect again." with Cancel and Forget buttons of equal width, Forget in the danger colour, and Escape or a click elsewhere SHALL cancel.

#### Scenario: A misclick on Forget
- **WHEN** the owner clicks Forget on the server's row by mistake and presses Escape
- **THEN** the server stays paired and its sessions stay in the list

#### Scenario: Forgetting a host
- **WHEN** the owner clicks Forget, then Forget again in the confirmation
- **THEN** the server leaves the Hosts menu and its sessions leave the list

### Requirement: Usage follows the selected session
The usage footer SHALL show the usage of the selected session's host, naming the host when it is not this computer, and this computer's usage when nothing is selected. The Usage window (CPU and memory) SHALL show the same host's sessions and name it when it is not this computer.

#### Scenario: A server session selected
- **WHEN** the owner selects the server's health agent
- **THEN** the footer shows the server's Claude and Codex usage, named "server"

### Requirement: New sessions on a chosen host
When more than one host is connected, New Session SHALL ask which host starts the session, the selected session's host first, and its folder browser SHALL show that host's folders.

#### Scenario: Starting on the server
- **WHEN** the owner opens New Session, chooses the server and a folder there
- **THEN** the session starts on the server and appears in the list with "server"
