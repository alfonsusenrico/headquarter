## ADDED Requirements

### Requirement: Sessions open on a phone
While a paired phone is the client in use for a sidebar session, its row SHALL show a phone mark in place of its age, with the tooltip "Open on <phone>", and SHALL keep its status word and glyph.

#### Scenario: Opened on the phone
- **WHEN** the owner opens "cash flow tracker dev" on the phone "Pixel 8" and the desktop shows another session
- **THEN** the row of "cash flow tracker dev" shows the phone mark, and its tooltip reads "Open on Pixel 8"

#### Scenario: Back on the desktop
- **WHEN** the desktop takes the session back, or the phone closes it
- **THEN** the row shows its age again
