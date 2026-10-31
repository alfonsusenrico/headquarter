## MODIFIED Requirements

### Requirement: Background and review look apart from working
The system SHALL show "background" in its own colour with a stacked-layers mark, and "review" in another colour with a diamond, both still, both apart from working's colour and movement and readable at 4.5:1 on the sidebar and the open row in every theme. Themes made from a palette SHALL take the colours from its cyan and magenta, and a theme file MAY set them.

#### Scenario: Side by side
- **WHEN** one session works, one shows "background" and one shows "review" in the built-in dark theme
- **THEN** "working" is blue with moving dots, "background" is teal with the stacked-layers mark, and "review" is violet with a diamond

#### Scenario: A template
- **WHEN** the owner uses the Nord template
- **THEN** "background" is Nord's cyan and "review" is Nord's magenta, each readable on the sidebar and the open row

#### Scenario: A theme file sets them
- **WHEN** the owner's theme file sets `background_work` in its interface colours
- **THEN** "background" uses that colour, and "review" keeps the colour made from the palette

#### Scenario: Background work after the turn
- **WHEN** a session at its prompt has a background agent running
- **THEN** its row ends with "background" and the stacked-layers mark where other rows show their glyph
