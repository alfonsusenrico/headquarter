## MODIFIED Requirements

### Requirement: Appearance settings
The system SHALL let the owner set, in the settings file and with Alacritty's names and colour format, the window opacity, blur, padding, and decorations, the font family, size, and styles, the cursor style, and the colours, including colours imported from a theme file. On macOS the window SHALL never show its window buttons, whatever the decorations setting says. The window opacity and blur SHALL apply to dark themes. Light themes SHALL draw opaque unless the owner sets `opacity_light`, Headquarter's own key, below 1.

#### Scenario: Owner's Alacritty look
- **WHEN** the settings file sets opacity 0.7, blur on, buttonless decorations, padding 10, a beam cursor, JetBrainsMono Nerd Font 14, and imports a Catppuccin Mocha colour file
- **THEN** the app window is translucent and blurred in a dark theme, has no window buttons or header strip, and the terminal uses that font, cursor, padding, and those colours

#### Scenario: A light theme over a dark desktop
- **WHEN** the settings file sets opacity 0.7 and blur on, the owner picks a light theme, and the desktop picture is dark
- **THEN** the window is opaque, its surfaces are the theme's light colours, and text meets AA contrast

#### Scenario: A translucent light window on purpose
- **WHEN** the settings file sets `opacity_light = 0.85` and the owner picks a light theme
- **THEN** the window is translucent at 85%, with blur when blur is on

#### Scenario: Blur unavailable
- **WHEN** blur is on and the platform's window system cannot blur
- **THEN** the window is translucent without blur and everything else applies

#### Scenario: Unsupported Alacritty key
- **WHEN** the settings file contains an Alacritty key Headquarter does not support
- **THEN** it is reported with its line as unsupported rather than silently ignored

#### Scenario: Full decorations on macOS
- **WHEN** the settings file sets `decorations = "full"` on macOS
- **THEN** the window shows no window buttons, is still dragged from the sidebar's top strip, and the setting is not reported as an error
