## MODIFIED Requirements

### Requirement: Reconnecting to a remote host
While a host is unreachable, the app SHALL keep that host's sessions in the list greyed with "Reconnecting…", retry by itself, and reattach its terminals when it returns, leaving the other hosts' sessions as they are. When the app gives up on a paired host (the host removed this computer, or the two run releases that cannot talk), it SHALL say so in the sidebar with the host's name and what to do, until the host lists its sessions again or is forgotten. When the releases differ, it SHALL keep trying every 30 seconds without the owner acting.

#### Scenario: Network drop
- **WHEN** the network to the server drops for 20 seconds
- **THEN** the server's sessions show greyed with "Reconnecting…", this computer's sessions stay as they are, and when the network returns the server's sessions and an open terminal on the server come back without the owner acting

#### Scenario: Removed on the host
- **WHEN** the owner removes this computer on the host
- **THEN** the app stops retrying that host, its sessions leave the list, and the sidebar and the Hosts menu say the host no longer knows this computer and to pair it again with Connect to a Host

#### Scenario: The host runs an older release
- **WHEN** the server runs Headquarter 0.2.0-alpha.2 (protocol 15) and the app 0.4.0-alpha.1 (protocol 16)
- **THEN** the sidebar says "<server> needs an update: it runs Headquarter 0.2.0-alpha.2, this app 0.4.0-alpha.1." and the Hosts menu shows the server as "needs an update" with that reason

#### Scenario: The app is the older side
- **WHEN** the server speaks a higher protocol than the app
- **THEN** the sidebar says "Update this app: <server> runs Headquarter <its version>, this app <app version>."

#### Scenario: The host is updated while the app runs
- **WHEN** the server needed an update and the owner updates it without touching the app
- **THEN** within about 30 seconds the server's sessions return to the list and the sidebar line goes away

#### Scenario: Opening with the host away
- **WHEN** the app opens and a paired host cannot be reached yet
- **THEN** this computer's sessions show at once, and the Hosts menu says the host is connecting

### Requirement: Hosts menu
The sidebar header SHALL open a Hosts menu listing this computer when it has a host and each paired host with its state (connected, connecting, reconnecting, needs an update, pair again, not reachable), Connect to a Host, and Forget for each paired host. A host the app gave up on SHALL show why under its row. While any paired host is not connected, the menu SHALL offer Reconnect all, which tries every such host again at once. Forget SHALL ask in place before forgetting: the row SHALL read "Forget <host>? You'll need a new code from it to connect again." with Cancel and Forget buttons of equal width, Forget in the danger colour, and Escape or a click elsewhere SHALL cancel.

#### Scenario: A misclick on Forget
- **WHEN** the owner clicks Forget on the server's row by mistake and presses Escape
- **THEN** the server stays paired and its sessions stay in the list

#### Scenario: Forgetting a host
- **WHEN** the owner clicks Forget, then Forget again in the confirmation
- **THEN** the server leaves the Hosts menu and its sessions leave the list

#### Scenario: Reconnect all
- **WHEN** the server shows "reconnecting" or "needs an update" and the owner chooses Reconnect all
- **THEN** the app tries the server again at once, and its sessions return as soon as it answers on the same release

#### Scenario: Every host connected
- **WHEN** every paired host is connected
- **THEN** the Hosts menu does not show Reconnect all

#### Scenario: Forgetting a host that needs an update
- **WHEN** the server needs an update and the owner forgets it
- **THEN** its line leaves the sidebar and the app stops trying it

### Requirement: Every host in one list
The sidebar SHALL list the sessions of this computer's host and of every paired host together. A session on another computer SHALL show that computer's name; on a computer without its own host, every session SHALL show its host's name. Every action on a session SHALL go to its host. When a row is too narrow for the title and the whole name, the name SHALL keep at least its first four characters before an ellipsis, and the title SHALL give way first.

#### Scenario: The Mac and the server
- **WHEN** the owner on the Mac has a server paired with a health agent running there
- **THEN** the sidebar lists the Mac's sessions without a host name and the health agent with "server"

#### Scenario: On Windows
- **WHEN** the owner on a Windows PC has one server paired
- **THEN** every session shows "server"

#### Scenario: Renaming a server session
- **WHEN** the owner renames the server's health agent from the Mac's sidebar
- **THEN** the server renames it, and the phone paired with the server shows the new name

#### Scenario: A long title on a selected row
- **WHEN** the selected row of "Server Management Agent Setup" on "msigs63vr" shows its close button
- **THEN** the tag reads at least "msig…", centred in its box, and the title is shortened instead
