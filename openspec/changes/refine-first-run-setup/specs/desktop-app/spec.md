# Spec Delta

## ADDED Requirements

### Requirement: Starting shows the agents checking in
The system SHALL show, until the host answers with its first session list, the agents' marks lighting up in turn above "Starting Headquarter…" in place of the empty sidebar and its hints.

#### Scenario: First seconds after opening
- **WHEN** the app opens and the host has not answered yet
- **THEN** the sidebar shows the marks lighting up in turn and the pane shows no hint, until the sessions are listed
