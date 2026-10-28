## REMOVED Requirements

### Requirement: In use on another device
**Reason**: The phone is the secondary device (owner, 2026-10-28). It no longer shows a cover when another device has the session; it leaves the session screen instead.
**Migration**: See "Leaving a session another device took".

## ADDED Requirements

### Requirement: Leaving a session another device took
The app SHALL take a running session or its shell when the owner opens it, without asking. When another client becomes the one in use for the open session, the app SHALL leave the session or shell screen for home and SHALL show for a few seconds "<session> is open on <computer>", or the other phone's name.

#### Scenario: The desktop takes the session back
- **WHEN** the phone shows "cash flow tracker dev" and the owner presses "Use here" on the desktop "MacBook"
- **THEN** the phone returns to home and shows "cash flow tracker dev is open on MacBook"

#### Scenario: Opening on the phone
- **WHEN** the desktop shows a session and the owner opens it on the phone
- **THEN** the phone shows the terminal at its width at once, with nothing to confirm
