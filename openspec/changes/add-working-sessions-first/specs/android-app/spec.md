## ADDED Requirements

### Requirement: Home rows hold still while touched
While a finger is on the home list, or the list is still moving after a fling, the app SHALL keep each listed session in its place within its list and SHALL still update its status and age. A session new to a list SHALL take its place by the desktop's order. Once the list is at rest and untouched, the app SHALL apply the desktop's order.

#### Scenario: A session starts working during a scroll
- **WHEN** the owner scrolls the home list and a session lower in its group starts working
- **THEN** its row shows working in its current place, and moves to the top of its group once the list is at rest and untouched

#### Scenario: Not touched
- **WHEN** the home list is untouched and a session starts working
- **THEN** its row moves to the top of its group as on the desktop
