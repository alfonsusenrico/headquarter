## MODIFIED Requirements

### Requirement: Update log
The system SHALL show under the update's steps a log of what the update has done, newest line last: the installer fetched and matched to its checksum, each archive the installer fetches and checks, what it installs where, and the restart. It SHALL leave out the installer's advice written for someone at a terminal. The log SHALL start at the top of its box, scroll by itself to keep the newest line in view, and let the owner scroll back through every line of the update without being pulled down while scrolled back.

#### Scenario: A whole update
- **WHEN** an update runs to the restart
- **THEN** the log names the installer's checksum match, both archives downloaded and matched, the commands and the app installed, and the host being stopped

#### Scenario: Terminal advice left out
- **WHEN** the installer prints how to open the app or to run `hq stop`
- **THEN** those lines do not appear in the log

#### Scenario: The first lines
- **WHEN** an update starts and the log holds two lines
- **THEN** they show at the top of the box

#### Scenario: Following the update
- **WHEN** the log holds more lines than fit and a new line arrives
- **THEN** the box shows the newest line at its bottom

#### Scenario: Reading back
- **WHEN** the owner scrolls up during an update and a new line arrives
- **THEN** the view stays where the owner put it, and following resumes once the owner scrolls back to the bottom
