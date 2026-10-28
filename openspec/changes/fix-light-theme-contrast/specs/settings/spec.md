## ADDED Requirements

### Requirement: Light themes designed for light
Light themes SHALL meet their contrast targets on every surface Headquarter draws (text 4.5:1, icons and marks 3:1), SHALL separate the sidebar, header and pane by surfaces or lines, and SHALL keep the agents' marks in their own colours. The terminal SHALL keep agents' text at a minimum contrast against its background in light themes, which the owner can change or turn off. Agents that accept a theme per launch SHALL be told whether Headquarter is light or dark, without changing their own configuration.

#### Scenario: Header in a light theme
- **WHEN** the owner picks the built-in light theme
- **THEN** the session header's title, folder, branch, model and figures each meet 4.5:1 against the header's surface

#### Scenario: Claude Code in a light theme
- **WHEN** a Claude Code session runs while Headquarter uses a light theme
- **THEN** its code spans and dimmed text meet at least 3:1 against the terminal background

#### Scenario: Dark themes unchanged
- **WHEN** the owner picks a dark theme
- **THEN** the chrome, the marks and the terminal look as they did before the change
