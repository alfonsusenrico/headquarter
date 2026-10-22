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
