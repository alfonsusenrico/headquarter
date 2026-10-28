## ADDED Requirements

### Requirement: Home layout
Home SHALL show a large title that folds away while the list scrolls down and returns at the top, a full-width row for the computer with its connection state that opens the switcher and keeps Settings in reach, usage as one tile per agent, and a floating Add button that shrinks to an icon while the list scrolls down. What Home lists and how it behaves SHALL stay as before.

#### Scenario: The title folds away
- **WHEN** the owner scrolls the session list down
- **THEN** the "Headquarter" title row folds away while the computer row and Settings stay, and the title returns when the list is back at the top

#### Scenario: The computer row
- **WHEN** the phone is reconnecting to a computer named "Studio Mac"
- **THEN** the computer row shows the dot, "Studio Mac" and "reconnecting…", and tapping it opens the computers sheet

#### Scenario: Usage tiles
- **WHEN** the computer reports usage for Claude Code, Codex and agy
- **THEN** Home shows three tiles of equal width, each with the agent, the window, the percentage and a bar

#### Scenario: Add while scrolling
- **WHEN** the owner scrolls the list down while connected
- **THEN** the Add button shrinks to a round + icon, and it shows "Add" again when the owner scrolls up or reaches the top

### Requirement: One look for the phone's screens
Home, Pairing, Settings and Add sessions SHALL use vector icons for back, settings, add, search and chevrons, with a content description on every icon button, and keep their flows and wording.

#### Scenario: Pairing steps
- **WHEN** the owner opens Pair another computer
- **THEN** three numbered steps show above the camera, and "Enter the code instead" switches to the code field as before

#### Scenario: Icons are named
- **WHEN** TalkBack reads the back button on Settings
- **THEN** it says "Back"
