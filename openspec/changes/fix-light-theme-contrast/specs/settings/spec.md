## ADDED Requirements

### Requirement: Light themes designed for light
Light themes SHALL meet their contrast targets on every surface Headquarter draws (text 4.5:1, icons and marks 3:1), SHALL separate the sidebar, header and pane by surfaces or lines, and SHALL keep the agents' marks in their own colours.

#### Scenario: Header in a light theme
- **WHEN** the owner picks the built-in light theme
- **THEN** the session header's title, folder, branch, model and figures each meet 4.5:1 against the header's surface

#### Scenario: Dark themes unchanged
- **WHEN** the owner picks a dark theme
- **THEN** the chrome and the marks look as they did before the change

### Requirement: Minimum contrast in the terminal
In light themes the terminal SHALL keep text an agent draws at a minimum contrast against its background by shifting the text's lightness, and the owner SHALL be able to change or turn off that minimum.

#### Scenario: Dark-theme colours on a light background
- **WHEN** an agent draws light grey text on the light terminal background
- **THEN** the text shows darker, at least at the minimum contrast, in the same hue

#### Scenario: Turned off
- **WHEN** the owner turns the minimum contrast off
- **THEN** the terminal draws the agent's colours exactly as sent

### Requirement: Agents told the mode
Agents that accept a theme per launch SHALL be told whether Headquarter is light or dark when they start, without any change to their own configuration.

#### Scenario: Claude Code in a light theme
- **WHEN** a Claude Code session starts while Headquarter uses a light theme
- **THEN** it draws its light colours, if it accepts a per-launch theme, and its code spans and dimmed text meet at least 3:1 against the terminal background either way
