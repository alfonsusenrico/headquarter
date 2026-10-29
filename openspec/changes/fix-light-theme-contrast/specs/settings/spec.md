## ADDED Requirements

### Requirement: Light themes designed for light
Light themes SHALL meet their contrast targets on every surface Headquarter draws (text 4.5:1, icons 3:1), SHALL separate the sidebar, header and pane by surfaces or lines, and SHALL keep the agents' marks, which are the vendors' brand marks, in their own colours.

#### Scenario: Header in a light theme
- **WHEN** the owner picks the built-in light theme
- **THEN** the session header's title, folder, branch, model and figures each meet 4.5:1 against the header's surface

#### Scenario: A light template
- **WHEN** the owner picks the Catppuccin Latte template
- **THEN** the sidebar sits on a surface darker than the terminal and its header, a line separates the header from the terminal, and the muted text, group headers, needs-you text and accent each meet 4.5:1 against the sidebar and the terminal background

#### Scenario: Dark themes unchanged
- **WHEN** the owner picks a dark theme
- **THEN** the chrome and the marks look as they did before the change

### Requirement: Minimum contrast in the terminal
In light and dark themes the terminal SHALL keep text an agent draws at a minimum contrast against its background by shifting the text's lightness, and the owner SHALL be able to change or turn off that minimum.

#### Scenario: Dark-theme colours on a light background
- **WHEN** an agent draws light grey text on the light terminal background
- **THEN** the text shows darker, at least at the minimum contrast, in the same hue

#### Scenario: Too dark on a dark background
- **WHEN** an agent draws dark blue text on the dark terminal background
- **THEN** the text shows lighter, at least at the minimum contrast, in the same hue

#### Scenario: Turned off
- **WHEN** the owner turns the minimum contrast off
- **THEN** the terminal draws the agent's colours exactly as sent

#### Scenario: The default minimum
- **WHEN** the owner has not set a minimum
- **THEN** every glyph meets 4.5:1 against its own background

### Requirement: Agents told the mode
Agents that accept a theme per launch SHALL be told whether Headquarter's terminal is light or dark when they start, including sessions the host starts with no window open, without any change to their own configuration. A plain light, dark or automatic theme the owner set in the agent SHALL follow the terminal; any other theme the owner set there SHALL win.

#### Scenario: Claude Code in a light theme
- **WHEN** a Claude Code session starts while Headquarter uses a light theme
- **THEN** it draws its light colours

#### Scenario: Started with the host
- **WHEN** the host restarts the owner's sessions before any window opens, and the last theme was light
- **THEN** Claude Code draws its light colours

#### Scenario: A plain dark theme in Claude Code
- **WHEN** the owner's Claude Code settings say `dark` and Headquarter uses a light theme
- **THEN** Claude Code draws its light colours

#### Scenario: A colour-blind theme in Claude Code
- **WHEN** the owner set Claude Code's `dark-daltonized` theme
- **THEN** Claude Code uses that theme, and Headquarter adds none
