## MODIFIED Requirements

### Requirement: Host switcher
The sidebar header SHALL name the host on screen and open the list of hosts: this computer when it has a host, each paired host with its state, Connect to a Host, and Forget for a paired host. Choosing one SHALL show it. Paired hosts SHALL stay connected while another host is on screen, so choosing one shows its sessions without dialling again.

#### Scenario: Switching to this computer
- **WHEN** the server is on screen and the owner chooses this computer in the switcher
- **THEN** the terminals close and the sidebar shows this computer's sessions

#### Scenario: Switching back without reconnecting
- **WHEN** the owner switches from the server to this computer and back within a minute
- **THEN** the server's sessions show without "Connecting to <host>…"

#### Scenario: Pairing a second host
- **WHEN** the server is paired and the owner pairs a second computer
- **THEN** the server stays connected, and switching back to it does not dial again
