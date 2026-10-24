# Spec Delta

## MODIFIED Requirements

### Requirement: Usage details
The system SHALL show, when the owner opens an agent's ring, each window with its percentage used and its reset time in local 12-hour time, including per-model windows the agent reports, and SHALL let the owner refresh the reading, keeping the details open and showing that the refresh is under way until the new readings are in. Clicking the ring that opened the panel, pressing Escape, or clicking outside SHALL close it.

#### Scenario: Codex details
- **WHEN** the owner opens the Codex ring
- **THEN** the popover lists the five-hour and weekly windows with bars, percentages and reset times, and a Refresh button, with no reading age

#### Scenario: Refresh
- **WHEN** the owner presses Refresh
- **THEN** the popover stays open, the button shows "Refreshing…" with a spinner until the reading is taken again, and the popover then shows the new numbers without being reopened

#### Scenario: Refresh just after a reading
- **WHEN** the owner presses Refresh within the refresh floor after a reading
- **THEN** no new reading is taken and the button returns to Refresh at once

#### Scenario: Unavailable agent
- **WHEN** the owner opens agy's ring
- **THEN** the popover states why its usage cannot be read and shows no percentage

#### Scenario: Closed by its own ring
- **WHEN** Claude's usage panel is open and the owner clicks Claude's ring again
- **THEN** the panel closes

#### Scenario: Per-model week
- **WHEN** Claude's `/usage` shows "Current week (Fable)" at 3%
- **THEN** Claude's details list "week · Fable" at 3% below the all-models week

#### Scenario: Long window name without a bar
- **WHEN** agy's details show "week · Gemini" as reset since the last reading, next to "week · Gemini 4 Argon" with a bar
- **THEN** both names stay on one line, and "reset since this reading" takes the remaining width
