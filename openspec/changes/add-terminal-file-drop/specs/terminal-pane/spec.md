# Spec Delta

## ADDED Requirements

### Requirement: Dropped files
The system SHALL type the paths of files and folders dropped on the terminal pane into the attached agent as a paste, each with shell characters escaped by a backslash, separated by spaces and followed by a space, and SHALL ignore drops anywhere else.

#### Scenario: Image onto a Claude Code prompt
- **WHEN** the owner drags `Screenshot 2026-10-23 at 14.29.43.png` from the Desktop onto a Claude Code session's pane
- **THEN** Claude Code receives `/Users/<owner>/Desktop/Screenshot\ 2026-10-23\ at\ 14.29.43.png ` as a bracketed paste and attaches the image

#### Scenario: Several files
- **WHEN** the owner drops two files on the pane at once
- **THEN** both paths are typed, separated by a space

#### Scenario: Dropped outside the pane
- **WHEN** the owner drops a file on the sidebar or on a pane with no attached session
- **THEN** nothing is typed
