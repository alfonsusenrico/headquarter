## ADDED Requirements

### Requirement: Quiet notices
The app SHALL show short messages in a notice at the bottom right of the window that slides in, leaves by itself after 5 seconds unless the pointer is on it, can be dismissed, never takes the keyboard, and appears without sliding when reduced motion is on. A notice reporting a failure SHALL stay until dismissed.

#### Scenario: A notice leaves by itself
- **WHEN** a notice appears and the owner keeps typing in the session
- **THEN** the keys reach the session, and the notice slides out after 5 seconds

#### Scenario: Dismissed
- **WHEN** the owner presses the notice's close button
- **THEN** it slides out at once
