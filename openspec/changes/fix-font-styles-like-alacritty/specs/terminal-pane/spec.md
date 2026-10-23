# Spec Delta

## ADDED Requirements

### Requirement: Line height from the font
The system SHALL space the pane's lines by the font's ascent plus descent, each rounded, as Alacritty does, and SHALL use 1.2 times the font size only when the font reports no metrics.

#### Scenario: JetBrains Mono at 14
- **WHEN** the settings file sets JetBrainsMono Nerd Font at size 14
- **THEN** the pane's lines are 18 points apart, as in Alacritty with the same settings
