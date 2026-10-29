# settings Specification

## Purpose
Gives the owner one optional file to adjust Headquarter's defaults, with mistakes reported precisely instead of silently ignored.

## Requirements

### Requirement: Optional settings file
The system SHALL read settings from the owner's configuration folder when the file exists and SHALL use built-in defaults when it does not.

#### Scenario: No settings file
- **WHEN** no settings file exists
- **THEN** every command works with all hostable agents enabled and the built-in look

### Requirement: Agent settings
The system SHALL let the owner pin an agent to a specific executable and turn an agent off.

#### Scenario: Pinned executable
- **WHEN** the settings file pins Codex to a specific executable and the owner starts a Codex session
- **THEN** that executable is used even if another Codex is found first elsewhere

#### Scenario: Agent turned off
- **WHEN** the settings file turns agy off and the owner starts an agy session
- **THEN** the request fails saying agy is turned off in the settings file

### Requirement: Precise errors
The system SHALL reject a settings file with unknown keys, wrong types, or out-of-range values, naming the file, the line, and the problem, and SHALL keep listing sessions while refusing to start new ones until the file is fixed.

#### Scenario: Typo in a key
- **WHEN** the settings file contains an unknown key
- **THEN** commands that need settings fail with a message naming the file, the line of the key, and that the key is unknown

#### Scenario: Running sessions stay visible
- **WHEN** the settings file is invalid while sessions are running
- **THEN** listing sessions still works and starting a session fails with the settings error

#### Scenario: Fixed without restart
- **WHEN** the owner fixes the settings file while the host runs
- **THEN** the next request uses the corrected settings

### Requirement: Appearance settings
The system SHALL let the owner set, in the settings file and with Alacritty's names and colour format, the window opacity, blur, padding, and decorations, the font family, size, and styles, the cursor style, and the colours, including colours imported from a theme file. On macOS the window SHALL never show its window buttons, whatever the decorations setting says.

#### Scenario: Owner's Alacritty look
- **WHEN** the settings file sets opacity 0.7, blur on, buttonless decorations, padding 10, a beam cursor, JetBrainsMono Nerd Font 14, and imports a Catppuccin Mocha colour file
- **THEN** the app window is translucent and blurred, has no window buttons or header strip, and the terminal uses that font, cursor, padding, and those colours

#### Scenario: Blur unavailable
- **WHEN** blur is on and the platform's window system cannot blur
- **THEN** the window is translucent without blur and everything else applies

#### Scenario: Unsupported Alacritty key
- **WHEN** the settings file contains an Alacritty key Headquarter does not support
- **THEN** it is reported with its line as unsupported rather than silently ignored

#### Scenario: Full decorations on macOS
- **WHEN** the settings file sets `decorations = "full"` on macOS
- **THEN** the window shows no window buttons, is still dragged from the sidebar's top strip, and the setting is not reported as an error

### Requirement: Dim unfocused
The system SHALL fade the pane or window that does not have focus by the configured amount, and SHALL not fade anything when the amount is zero.

#### Scenario: Focus moves to the sidebar
- **WHEN** dimming is set and the owner moves focus from the terminal pane to the session list
- **THEN** the terminal pane fades by that amount until it has focus again

### Requirement: Keyboard settings
The system SHALL let the owner make the Option key act as Alt, on the left, right, or both sides, and bind keys to send given text.

#### Scenario: Shift+Enter for a newline
- **WHEN** a binding sends `ESC` followed by Enter for Shift+Enter and the owner presses Shift+Enter in a Claude session
- **THEN** Claude receives those bytes and inserts a newline instead of submitting

### Requirement: Live appearance changes
The system SHALL apply appearance and keyboard settings when the settings file changes, without restarting the app.

#### Scenario: Opacity edited
- **WHEN** the owner changes the opacity in the settings file while the app is open
- **THEN** the window shows the new opacity

### Requirement: Notification settings
The system SHALL let the owner turn "needs you" and "replied" notifications on or off in the settings file, both on by default, applied without a restart.

#### Scenario: Replied turned off
- **WHEN** the settings file sets `[notifications] replied = false`
- **THEN** finished turns no longer notify and "needs you" still does

### Requirement: Font styles as Alacritty reads them
On macOS, the system SHALL draw the bold, italic and bold italic styles with the font face whose style name equals the configured style exactly, SHALL draw a style with the normal face (normal weight, upright) when the font has no face of that name, as Alacritty does, saying so only in the app log.

#### Scenario: Style missing from the font
- **WHEN** the settings file sets JetBrainsMono Nerd Font Medium with bold "Heavy", and the font has no Heavy face
- **THEN** bold text is drawn in Medium like the rest, no note is shown, and the app log says that "Heavy" is not in the font

#### Scenario: Style the font has
- **WHEN** the settings file sets bold "ExtraBold" for JetBrainsMono Nerd Font
- **THEN** bold text is drawn with the ExtraBold face and no note is shown

### Requirement: Update settings
The system SHALL check for updates automatically unless the settings file sets `[updates] check = false`, applied without a restart; Check for Updates… SHALL work either way.

#### Scenario: Checks turned off
- **WHEN** the settings file sets `[updates] check = false`
- **THEN** no automatic check runs, and Check for Updates… still checks

### Requirement: Remote control settings
The system SHALL turn remote control on only when the settings file sets `[remote] enabled = true`, off by default, and SHALL use n0's public relays unless `[remote] relays` lists the owner's own relay URLs, which it SHALL then use instead.

#### Scenario: Default
- **WHEN** the settings file has no `[remote]` section
- **THEN** remote control is off and the host opens no network endpoint

#### Scenario: Own relay
- **WHEN** the settings file sets `[remote] relays = ["https://relay.example.com"]` and remote control is on
- **THEN** the host and its pairing codes use that relay and no other

#### Scenario: Invalid relay
- **WHEN** `[remote] relays` holds something that is not an https URL
- **THEN** the settings error names the file, the line and the problem, as for other settings

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
