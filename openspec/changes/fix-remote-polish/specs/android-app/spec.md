## MODIFIED Requirements

### Requirement: Connected only while on screen
The app SHALL connect while it is on screen, disconnect 30 seconds after it leaves the screen, and reconnect by itself when it returns or when the connection drops, keeping the last known list readable while it reconnects. Its banner SHALL say "Reconnecting to <computer>…" while it retries, and "<computer> is offline" once the computer has not answered for 2 minutes, each followed by the list's age when the phone has a list.

#### Scenario: Network lost
- **WHEN** the phone loses its network while showing home
- **THEN** the app shows Reconnecting with the last known list dimmed and its age, and resumes once the network returns

#### Scenario: Background
- **WHEN** the owner leaves the app for a minute
- **THEN** the computer no longer lists the phone as connected, and returning reconnects it

#### Scenario: Computer turned off
- **WHEN** the phone has tried for more than 2 minutes to reach "MacBook", whose list it last had 5 minutes ago
- **THEN** the banner reads "MacBook is offline · 5m"
