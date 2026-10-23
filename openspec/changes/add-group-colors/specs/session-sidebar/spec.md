# Spec Delta

## ADDED Requirements

### Requirement: Group colours
The system SHALL give each group a colour from a soft eight-colour palette suited to the light or dark mode, shown as a dot before the group's name and as a thin line along its rows; SHALL choose it automatically from the group's identity until the owner picks one from the dot; and SHALL let the owner return a group to automatic. A group's name SHALL always be shown with its colour.

#### Scenario: New groups
- **WHEN** the owner creates two groups one after the other and picks no colour
- **THEN** they get the first two palette colours in order, and keep them when the groups are reordered or renamed

#### Scenario: Picking a colour
- **WHEN** the owner clicks a group's dot and chooses "Green"
- **THEN** the dot and the line along the group's rows turn green, and stay green after the app and the host restart

#### Scenario: Back to automatic
- **WHEN** the owner chooses "Automatic" in that popover
- **THEN** the group shows its automatic colour again

#### Scenario: Collapsed group
- **WHEN** a coloured group is collapsed
- **THEN** its header keeps the dot, and its chip in the icon rail is outlined in its colour
