# Faster terminal drawing on the phone

Status: approved by the owner on 2026-10-27 ("approve the row cache").

## Why

The add-android-app verification found that drawing the terminal costs about 80 ms a frame on
the dev phone, and proposed caching each row's text layout. Measured on 2026-10-27
(`research/evidence/cache-terminal-row-layouts/findings.md`): while output scrolls, the terminal
takes about 111 ms a frame on the UI thread and the phone shows about 8 frames a second (median
frame 200 ms). With a still screen and a spinner it takes about 25 ms.

The cause is in `TerminalView`: each span is drawn with `drawText(measurer, text, topLeft)`,
whose layout constraints come from the space left after `topLeft`. The constraints are part of
the text cache's key, so every span that moves or sits at another column is laid out again,
about 500 layouts a frame while output scrolls.

## Expected Outcome

- Fast agent output scrolls smoothly on the phone: on the dev phone, the terminal's drawing
  falls from about 111 ms to about 16 ms a frame while output scrolls, and the phone shows
  about 24 frames a second instead of 8 (measured with experiment B in the findings).
- A still screen with a spinner costs about 14 ms a frame instead of 25.
- The terminal looks the same: same grid, colours, styles, cursor, wide characters.

## What Changes

- `TerminalView` keeps one text layout per row while the row's content is unchanged, drawn at
  the row's position, so rows that only move are not laid out again. A row with wide characters
  keeps one layout per span, placed at its column, also kept while unchanged.
- The cell width becomes the font's exact advance (measured over 100 characters) instead of a
  rounded "M", so a row drawn as one text lines up with backgrounds and the cursor.

## Non-goals

- The core's snapshot (`phone.screen()`, 2-3 ms) and the home refresh per change (about 1 ms).
- Drawing only rows that changed (partial invalidation), or recording rows as pictures: a
  further step if 14-16 ms is not enough.
- The phone UI redesign (separate options page).

## Capabilities affected

`android-app` (Session on the phone). No new project capability.

## Impact

`mobile/android/app/src/main/java/com/alfonsusenrico/headquarter/ui/TerminalView.kt` and a unit
test. No protocol, core or data change.
