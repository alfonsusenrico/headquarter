# Spec Delta

## ADDED Requirements

### Requirement: Font styles as Alacritty reads them
On macOS, the system SHALL draw the bold, italic and bold italic styles with the font face whose style name equals the configured style exactly, SHALL draw a style with the normal face (normal weight, upright) when the font has no face of that name, as Alacritty does, saying so only in the app log.

#### Scenario: Style missing from the font
- **WHEN** the settings file sets JetBrainsMono Nerd Font Medium with bold "Heavy", and the font has no Heavy face
- **THEN** bold text is drawn in Medium like the rest, no note is shown, and the app log says that "Heavy" is not in the font

#### Scenario: Style the font has
- **WHEN** the settings file sets bold "ExtraBold" for JetBrainsMono Nerd Font
- **THEN** bold text is drawn with the ExtraBold face and no note is shown
