## ADDED Requirements

### Requirement: Keyboard Shortcuts window
The app SHALL list every app shortcut in a Keyboard Shortcuts window, opened with ⌘/ on macOS (Ctrl+Shift+/ on Linux) or Headquarter ▸ Keyboard Shortcuts… on macOS, each with the keys for the system it runs on and a one-line explanation, taken from the same table the app binds its keys from. Typing SHALL filter the list at once.

#### Scenario: Opening the list
- **WHEN** the owner presses ⌘/ on macOS
- **THEN** a window lists the app's shortcuts by area, each with its keys and what it does

#### Scenario: Typing filters
- **WHEN** the window is open and the owner types "shell"
- **THEN** only the shortcuts whose name, explanation or key matches remain, and Esc shows them all again

#### Scenario: Linux keys
- **WHEN** the app runs on Linux and the window is open
- **THEN** each shortcut shows Ctrl Shift with its key
