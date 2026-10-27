# Design: a Keyboard Shortcuts window

## Context

Shortcuts are decided in `shortcuts.rs` (`shortcut(key, keys, mac)`) and bound for the menus in
`menus.rs` (`BOUND`, macOS only). Sidebar keys (arrows, Enter, Alt-Shift arrows, Esc) live in the
sidebar's key handling. The options page lists them all (section 3).

## Decisions

### D1. One table

`shortcuts.rs` gets a `SHORTCUTS` table: the shortcut, its key, its area, its name and its
explanation. `shortcut()` and `menus.rs` read keys from it, and so does the window. Sidebar keys
are listed from a second, fixed table in the same file, marked as sidebar keys. A test checks
every `Shortcut` value appears in the table once, and no two share a key.

### D2. The window

In the modal frame the ⌘O and theme windows use, about 560 px wide: area headings, then rows
with key chips on the left (⌘ N on macOS, Ctrl Shift N on Linux) and the name with its
explanation on the right. Typing any character shows a filter line at the top with the typed
text and narrows the rows to those whose name, explanation or key matches. Backspace edits it.
Esc clears a filter, then closes. ⌘/ again closes.

### D3. Opening it

⌘/ on macOS, Ctrl+Shift+/ on Linux (also matching `?`, as the code matches `!` for 1). On macOS
also Headquarter ▸ Keyboard Shortcuts…, beside Themes….

## Risks

- A future shortcut added outside the table would be missing from the window. The test in D1
  covers the `Shortcut` values, not keys handled elsewhere.
