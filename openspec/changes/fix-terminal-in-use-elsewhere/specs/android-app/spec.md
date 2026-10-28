## ADDED Requirements

### Requirement: In use on another device
While another client is in use for the open session, the app SHALL cover the terminal with "In use on <computer>", or "In use on <phone>" when another phone is in use, and a "Use here" button, and SHALL keep the header, the key row and the text field usable. Tapping "Use here", typing, or pressing a key SHALL make the phone the client in use, after which the cover goes away and the agent redraws at the phone's width.

#### Scenario: The desktop takes the session
- **WHEN** the phone shows "cash flow tracker dev" and the owner opens it on the desktop "MacBook"
- **THEN** the phone covers the terminal with "In use on MacBook" and "Use here", and no garbled text is visible

#### Scenario: Use here
- **WHEN** the owner taps "Use here"
- **THEN** the cover goes away and the agent redraws at the phone's width

#### Scenario: Typing takes over
- **WHEN** the terminal is covered and the owner types into the text field and sends it
- **THEN** the text reaches the agent, the phone becomes the client in use, and the cover goes away
