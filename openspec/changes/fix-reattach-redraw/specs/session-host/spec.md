## ADDED Requirements

### Requirement: Redraws the agent notices
When a viewer attaches to a session or takes it over, the system SHALL change the agent's terminal size in two steps the agent sees separately, one row fewer and then the viewer's size, restoring the size once the agent has repainted at the smaller size (it erases the screen) or after 500 ms, so the agent repaints at the viewer's size.

#### Scenario: Attach at the agent's current size
- **WHEN** the app attaches to a session at the size the agent already has
- **THEN** the agent sees one row fewer and then its size again, and repaints at that size

#### Scenario: Two attaches in quick succession
- **WHEN** two viewers attach to a session within 100 ms
- **THEN** the agent ends at the size of the viewer that attached last

#### Scenario: Agent writes nothing after a resize
- **WHEN** a viewer attaches to a shell that prints nothing after a size change
- **THEN** the size is restored after 500 ms
