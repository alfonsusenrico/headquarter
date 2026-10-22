# first-run-setup Specification

## Purpose
Gets a new owner from first launch to a ready sidebar: choosing the agents to use and a look, and writing the settings file safely, without touching any agent's own configuration.

## Requirements

### Requirement: First launch shows setup
The system SHALL show setup instead of the sidebar when the app starts and no settings file exists, and SHALL show it on request when one exists.

#### Scenario: Fresh machine
- **WHEN** the owner opens the app and has no Headquarter settings file
- **THEN** setup appears with the agents step

#### Scenario: Already set up
- **WHEN** a settings file exists and the owner opens the app normally
- **THEN** the sidebar appears and setup does not

### Requirement: Choose agents
The system SHALL list every known agent with its path, version and support, tick installed hostable agents by default, and turn off in the settings file the ones the owner unticks.

#### Scenario: Owner skips agy
- **WHEN** Claude, Codex and agy are installed and the owner unticks agy and finishes setup
- **THEN** the settings file turns agy off and Claude and Codex stay available for new sessions

#### Scenario: Agent not supported yet
- **WHEN** Gemini CLI is installed
- **THEN** it is listed greyed as not yet supported and cannot be ticked

### Requirement: Choose a look
The system SHALL offer the reference look by default and, when an Alacritty settings file exists, the owner's Alacritty look, naming which Alacritty settings carry over and which do not.

#### Scenario: Alacritty look chosen
- **WHEN** the owner chooses "Use my Alacritty look" and finishes setup
- **THEN** the settings file holds the window, font, cursor, colour and keyboard settings from the Alacritty file, with its theme import kept, and the app shows that look

#### Scenario: Unsupported Alacritty settings
- **WHEN** the Alacritty file has an `[env]` section
- **THEN** the look step names `env` as not carried over

#### Scenario: No Alacritty file
- **WHEN** no Alacritty settings file exists
- **THEN** only the reference look is offered

### Requirement: Safe writing
The system SHALL write a settings file that it reads back without errors or warnings, SHALL never replace a settings file that appeared during first-run setup, and, when setup is run again on an existing file, SHALL keep a dated copy and change only the settings setup asks about.

#### Scenario: Rerun keeps other settings
- **WHEN** the owner's settings file pins Codex's path, sets `dim_unfocused`, and has comments, and the owner runs setup again and turns agy off
- **THEN** the file turns agy off, the Codex path, `dim_unfocused` and the comments are unchanged, and a dated copy of the previous file is kept and named on the last step

#### Scenario: Look changed back
- **WHEN** the owner earlier imported their Alacritty look, then changed the opacity by hand, and runs setup again choosing the Headquarter look
- **THEN** the imported look settings are removed except the changed opacity, which stays and is listed on the last step

#### Scenario: Edit would not parse
- **WHEN** the edited file would not read back without errors
- **THEN** nothing is written and the error is shown

#### Scenario: File appeared meanwhile
- **WHEN** a settings file is created by other means while first-run setup is open
- **THEN** finishing setup does not replace it and says so

#### Scenario: Skipped
- **WHEN** the owner presses Skip on first run
- **THEN** a settings file with the defaults is written and setup does not appear on the next launch

### Requirement: Ready to fill the sidebar
The system SHALL open the sidebar with "Add sessions" open when setup finishes.

#### Scenario: Setup finished
- **WHEN** the owner finishes setup
- **THEN** the sidebar is shown with "Add sessions" open

### Requirement: Agent files untouched
The system SHALL NOT read credentials from or write to any agent's configuration during setup.

#### Scenario: Checksums
- **WHEN** setup has run to the end
- **THEN** the agents' configuration files are byte-for-byte unchanged
