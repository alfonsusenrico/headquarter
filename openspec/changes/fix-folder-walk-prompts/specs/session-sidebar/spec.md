# Spec Delta

## ADDED Requirements

### Requirement: Folder search leaves media libraries alone
The system SHALL NOT read the Music, Movies or Pictures folders directly in the home folder, nor package folders such as apps and media libraries, while it searches for project folders.

#### Scenario: Opening the New session window
- **WHEN** the owner opens the New session window on macOS
- **THEN** macOS does not ask whether Headquarter may access Apple Music, the photo library or videos

#### Scenario: A project's own media folder
- **WHEN** a project folder contains a folder named `Music`
- **THEN** that folder is still found by name
