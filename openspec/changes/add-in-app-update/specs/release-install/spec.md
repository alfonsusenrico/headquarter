## MODIFIED Requirements

### Requirement: macOS menu bar
On macOS the app SHALL show its name, Headquarter, in the menu bar, the Dock and the app switcher when installed, and SHALL offer a Headquarter, an Edit and a Window menu whose items run the same actions as the app's shortcuts, listing only shortcuts the app already keeps for itself, so no key an agent uses is taken.

#### Scenario: Installed app
- **WHEN** the owner opens Headquarter.app
- **THEN** the menu bar reads "Headquarter" with About Headquarter, Check for Updates…, Themes (⌘G), Hide (⌘H), Hide Others and Quit (⌘Q), an Edit menu with Copy and Paste, and a Window menu with Minimize (⌘M), New Session (⌘N) and Open Sessions (⌘O)

#### Scenario: Menu item
- **WHEN** the owner chooses Window > New Session
- **THEN** the New session window opens, as with ⌘N

#### Scenario: Agent keys
- **WHEN** an agent is focused and the owner presses a plain Ctrl key, such as Ctrl-C
- **THEN** the key reaches the agent and no menu item runs

#### Scenario: Linux
- **WHEN** the app runs on Linux
- **THEN** there is no app menu bar and the Ctrl-Shift shortcuts work unchanged

## ADDED Requirements

### Requirement: Installer progress
The installer SHALL, when `HQ_PROGRESS=1` is set, print one line per step (`progress download`, `progress verify`, `progress install`, `progress done <version>`) besides its usual output, and SHALL print nothing new without it.

#### Scenario: Progress asked for
- **WHEN** the installer runs with `HQ_PROGRESS=1`
- **THEN** its output holds `progress download`, `progress verify`, `progress install` and `progress done 0.1.9` in that order

#### Scenario: Not asked for
- **WHEN** the installer runs without `HQ_PROGRESS`
- **THEN** its output has no progress lines
