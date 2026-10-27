# A Keyboard Shortcuts window

Status: approved by the owner on 2026-10-27 ("approve all three, go ahead").

## Why

The owner wants one place that lists every shortcut (2026-10-27): "a modal showing a table,
the shortcut key combination on the left, the menu and a concise explanation on the right".
Shortcuts may become configurable later, not now. Options page:
`research/prototypes/folder-and-usage-options/index.html`, section 3. The owner chose A and B
combined: the table by default, and typing filters it at once.

## Expected Outcome

- ⌘/ (Ctrl+Shift+/ on Linux) or Headquarter ▸ Keyboard Shortcuts… opens a window listing
  every app shortcut, grouped (Sessions, Window, Terminal, Sidebar), each with its keys for
  this system and a one-line explanation.
- Typing anything filters the list by action or key at once, and Esc clears the filter, then
  closes the window.
- The list comes from the same table the app binds its keys from, so it cannot drift from the
  real keys. That table is where configurable shortcuts would start later.

## What Changes

- `hq-app` `shortcuts.rs`: one table of shortcuts with their action name, area and explanation.
  The key handling and the menus keep reading the same keys from it.
- `hq-app` `modal/`: the Keyboard Shortcuts window in the existing modal frame.
- `hq-app` `menus.rs`: Headquarter ▸ Keyboard Shortcuts… ⌘/ (macOS). Linux has no menu bar, so
  there it is the key alone.

## Non-goals

- Changing any shortcut, or making them configurable.
- The terminal's own keys, such as copy on selection.

## Capabilities affected

`desktop-app`. No new project capability.

## Impact

App only. No host or protocol change.
