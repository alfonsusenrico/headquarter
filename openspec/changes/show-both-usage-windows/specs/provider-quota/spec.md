# Spec Delta

## MODIFIED Requirements

### Requirement: Usage strip
The system SHALL show, under the sidebar, centred usage marks for the enabled agents nearest their limits, as many as fit on one line, each with an outer ring filled to the share used of the agent's longest current window (the week) and an inner ring for its shortest (5 hours) when it has two, per-model windows left to the details, SHALL fold the rest into a "+N" chip that opens every agent's usage, SHALL draw a ring red when its window has used 80% or more, and SHALL, from 80%, name the one agent near its limit with its nearest window (the shorter window when the whole percentages are equal) or, when several agents are, say how many.

#### Scenario: Three agents
- **WHEN** Claude, Codex and agy are enabled and Codex has used 82% of its five-hour window and 30% of its week
- **THEN** three centred marks are shown, Codex's first with its inner ring at 82% and its outer ring at 30%, agy's dotted, and the label reads "codex 82% · 5h"

#### Scenario: Several agents near their limits
- **WHEN** Claude has used 91% of its week and Codex 85% of its five hours
- **THEN** both rings are red and the label reads "2 agents near their limit"

#### Scenario: One agent, both windows near the limit
- **WHEN** only Claude is near its limit, with 85% of its five hours and 92% of its week used
- **THEN** both of Claude's rings are red and the label reads "claude 92% · week"

#### Scenario: Equal shares
- **WHEN** Claude's five hours and week both show 88%
- **THEN** the label reads "claude 88% · 5h"

#### Scenario: Below the threshold
- **WHEN** no agent has used 80% of any window
- **THEN** no label is shown and no ring is red

#### Scenario: One window
- **WHEN** an agent reports a single current window
- **THEN** its mark has only the outer ring, filled to that window

#### Scenario: Narrow sidebar
- **WHEN** the rings do not fit on one line
- **THEN** the agents furthest from their limits fold into a "+N" chip, and every agent stays reachable from it
