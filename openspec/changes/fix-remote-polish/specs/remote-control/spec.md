## ADDED Requirements

### Requirement: Pairing from a terminal
`hq remote pair` SHALL draw its QR code dark on light whatever the terminal's colours when it prints to a terminal, plain when piped or when `NO_COLOR` is set, and SHALL print the latest phone app's download address with the code.

#### Scenario: A dark terminal theme
- **WHEN** the owner runs `hq remote pair` in a terminal with light text on a dark background
- **THEN** the QR code shows black modules on white and a phone's camera reads it

#### Scenario: Getting the phone app from a server
- **WHEN** the owner runs `hq remote pair` over SSH on a server
- **THEN** "Get the phone app:" and the latest `headquarter-android.apk` address show with the code
