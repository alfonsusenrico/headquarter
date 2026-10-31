## ADDED Requirements

### Requirement: Usage total
The Usage popover and the Usage window SHALL end with Headquarter's own figures, all running sessions' figures together, and their sum, each with CPU and memory, counting every running session even where the popover lists only some.

#### Scenario: Popover total
- **WHEN** the owner opens the Usage popover with 12 sessions running
- **THEN** below the five busiest it shows Headquarter, all 12 sessions and the total, with CPU and memory for each
