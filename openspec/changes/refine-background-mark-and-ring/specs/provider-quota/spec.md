## ADDED Requirements

### Requirement: Near the limit stays visible when a reading is old
While a reading is old or its last update failed, a usage ring at or above the near-limit share SHALL keep the near-limit colour, softened, and other rings SHALL turn grey.

#### Scenario: An old reading near the limit
- **WHEN** Claude's week is at 93% and the last update failed
- **THEN** the week ring is a pale red, not grey, and the details say "Couldn't update since" with the time
