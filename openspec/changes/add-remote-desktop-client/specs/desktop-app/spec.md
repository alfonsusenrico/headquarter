## MODIFIED Requirements

### Requirement: Client of the host
The system SHALL connect as a client to this computer's host, starting it when needed, or to a paired host on another computer, and SHALL leave every session running when the window or the app closes.

#### Scenario: App closed mid-turn
- **WHEN** the owner quits the app while an agent is working
- **THEN** the agent keeps working and `hq ls` shows it

#### Scenario: Remote host on screen
- **WHEN** the owner chooses a paired host in the host switcher
- **THEN** the sidebar, terminals and usage show that host's sessions

## ADDED Requirements

### Requirement: Connect to a Host
The app SHALL pair with a host on another computer from a pairing code entered in Connect to a Host, keep the host in its list, and say plainly when the code is wrong, expired or the host unreachable.

#### Scenario: Pairing from the code
- **WHEN** the owner pastes a valid code from the server's `hq remote pair`
- **THEN** the app pairs, adds the server to its host list, and shows its sessions

#### Scenario: Expired code
- **WHEN** the owner enters a code that has expired
- **THEN** the window says the code expired and asks for a new one, and nothing is added

### Requirement: Host switcher
The sidebar header SHALL name the host on screen and open the list of hosts: this computer when it has a host, each paired host with its state, Connect to a Host, and Forget for a paired host. Choosing one SHALL show it.

#### Scenario: Switching to this computer
- **WHEN** the server is on screen and the owner chooses this computer in the switcher
- **THEN** the terminals close and the sidebar shows this computer's sessions

### Requirement: Reconnecting to a remote host
While a remote host is unreachable, the app SHALL say it is reconnecting, keep the last list greyed, retry by itself, and reattach the terminals when the host returns.

#### Scenario: Network drop
- **WHEN** the network drops for 20 seconds with the server on screen
- **THEN** the sidebar says "Reconnecting to <host>…", and when the network returns the sessions and the open terminal come back without the owner acting

### Requirement: A remote host is never stopped from the app
Quitting or updating the app with a remote host on screen SHALL act on this computer only: quit without the stop question, and update without stopping the remote host.

#### Scenario: Quit with the server on screen
- **WHEN** the owner quits the app while the server's sessions are on screen
- **THEN** the app quits without asking to stop the host, and the server's sessions keep running

### Requirement: Files stay on their computer
With a remote host on screen, dropping files into a terminal SHALL show a notice that the files are on this computer, and file links SHALL open only when the host is this computer.

#### Scenario: Dropping a file onto a remote session
- **WHEN** the owner drops a file from the PC onto the server's session
- **THEN** a notice says the file is on this computer and nothing is typed into the session
