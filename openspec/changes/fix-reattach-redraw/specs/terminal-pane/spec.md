## ADDED Requirements

### Requirement: Attach at the pane's size
The app SHALL attach to a session, or its shell, at the size its pane is laid out with, and SHALL NOT have the agent draw at a size the pane does not show. When the pane has no size 500 ms after opening, it SHALL attach at 80 columns and 24 rows.

#### Scenario: Coming back to a session
- **WHEN** the owner clicks a session in the sidebar that the window showed earlier
- **THEN** the app attaches at the pane's columns and rows, and the agent draws only at that size

#### Scenario: Coming back while the agent works
- **WHEN** the owner comes back to a session whose agent is producing output
- **THEN** the pane shows the agent's current screen with no text from another size
