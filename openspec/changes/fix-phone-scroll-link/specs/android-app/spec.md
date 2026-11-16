## ADDED Requirements

### Requirement: Scrolling keeps the link
Scrolling a full-screen agent on the phone SHALL keep the phone's link to the computer connected, over a relay as well as directly, and the terminal SHALL keep following the drag without freezing.

#### Scenario: A minute of scrolling Claude Code
- **WHEN** the owner drags up and down for a minute on a Claude Code session in fullscreen mode, over n0's relay, while the Mac app shows the same session
- **THEN** the phone stays connected, the conversation follows the drag, and the screen is current within a second of the last drag

### Requirement: Reattaching at the phone's size
After the phone reconnects to a session that another screen used at a different size, the phone SHALL show the session at the phone's own size and SHALL NOT show the other screen's layout squeezed into the phone's columns.

#### Scenario: The Mac app showed the session
- **WHEN** the phone reconnects to a Claude Code session the Mac app showed in a wide window
- **THEN** the phone's terminal shows Claude Code laid out for the phone's width

### Requirement: The phone logs why a link ended
When the phone's link to a computer ends, the app SHALL log why (lost after a read error, a write error or a refused frame; stopped by the app; given up) with the path and how long it was connected, and SHALL NOT log session names, typed input or terminal output.

#### Scenario: A lost link
- **WHEN** the phone's link ends after a read error
- **THEN** the phone's log has one line naming the read error, the path and the time connected, and no content
