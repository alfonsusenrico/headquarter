# Spec Delta

## MODIFIED Requirements

### Requirement: Choose agents
The system SHALL list only the agents Headquarter can host, each with its path and version or as not installed, followed by a note that more agents are on the way, tick installed ones by default, and turn off in the settings file the ones the owner unticks.

#### Scenario: Owner skips agy
- **WHEN** Claude, Codex and agy are installed and the owner unticks agy and finishes setup
- **THEN** the settings file turns agy off and Claude and Codex stay available for new sessions

#### Scenario: Agent not supported yet
- **WHEN** Gemini CLI is installed
- **THEN** it is not listed, and the step ends with "More agents are on the way."

### Requirement: Choose a look
The system SHALL offer the themes of the theme switcher, with the theme in use (System by default) chosen, show each theme on the setup screen as the owner chooses it, keep the chosen theme as the app's theme, and, when an Alacritty settings file exists, offer "Your Alacritty look" among them, naming which Alacritty settings do not carry over.

#### Scenario: Alacritty look chosen
- **WHEN** the owner chooses "Your Alacritty look" and finishes setup
- **THEN** the settings file holds the window, font, cursor, colour and keyboard settings from the Alacritty file, with its theme import kept, and the app shows that look

#### Scenario: Unsupported Alacritty settings
- **WHEN** the Alacritty file has an `[env]` section
- **THEN** the look step names `env` as not carried over

#### Scenario: No Alacritty file
- **WHEN** no Alacritty settings file exists
- **THEN** the themes are offered without "Your Alacritty look"

#### Scenario: A template chosen
- **WHEN** the owner chooses one of the templates and finishes setup
- **THEN** the setup screen showed that theme while it was chosen, and the app opens in it
