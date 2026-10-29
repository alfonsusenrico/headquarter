## ADDED Requirements

### Requirement: Errors read plainly
The app SHALL show each error from the phone's core as the sentence the core gives, with no field names, and SHALL name a lost connection when the core reports one without a reason.

#### Scenario: Pairing fails
- **WHEN** pairing fails because the computer did not answer
- **THEN** the app shows "Couldn't reach the computer: the computer did not answer", without "reason="

#### Scenario: Not connected
- **WHEN** an action fails because the phone is not connected
- **THEN** the app shows "Not connected to the computer."
