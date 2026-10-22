# Spec Delta

## MODIFIED Requirements

### Requirement: Usage strip
The system SHALL show, under the sidebar, centred usage marks for the enabled agents nearest their limits, as many as fit on one line, each with an outer ring filled to the share used of the agent's longest current window (the week) and an inner ring for its shortest (5 hours) when it has two, per-model windows left to the details, SHALL fold the rest into a "+N" chip that opens every agent's usage, and SHALL name the agent nearest its limit when one has used at least half of a window.

#### Scenario: Three agents
- **WHEN** Claude, Codex and agy are enabled and Codex has used 82% of its five-hour window and 30% of its week
- **THEN** three centred marks are shown, Codex's first with its inner ring at 82% and its outer ring at 30%, agy's dotted, and the label reads "codex 82% · 5h"

#### Scenario: One window
- **WHEN** an agent reports a single current window
- **THEN** its mark has only the outer ring, filled to that window

#### Scenario: Narrow sidebar
- **WHEN** the rings do not fit on one line
- **THEN** the agents furthest from their limits fold into a "+N" chip, and every agent stays reachable from it
