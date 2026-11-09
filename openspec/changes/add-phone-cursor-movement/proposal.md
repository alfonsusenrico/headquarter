# Move the cursor in the agent's box from the phone

Status: approved by the owner 2026-11-09 (chose "← → keys + tap" for moving the cursor).

## Why

The owner (2026-11-09), typing on the phone: there is no way to move the cursor in the agent's
input box. The key row has only ↑ and ↓, and tapping the terminal only opens the keyboard
(`refine-phone-typing-and-links` kept the cursor at the end on purpose).

Observed 2026-11-09 without sending anything: Claude Code 2.1.319 and Codex 0.166.0 both keep the
terminal's own cursor visible at their input position, move it with ← and →, and insert typed text
there (`research/evidence/phone-typing-images/2026-11-09-cursor.md`).

## Expected Outcome

- ← and → join the key row, in sessions and in the shell.
- Tapping on the agent's input line puts the cursor where the owner tapped; typing and Backspace
  then work there.

## What Changes

- Key rows: a session's grid becomes six a row (1 2 3 Esc ⇧Tab Tab / ← ↑ ↓ → Ctrl-C and the image
  key); a shell's four a row (Esc Tab Ctrl-C Ctrl-D / ← ↑ ↓ →) (D1).
- `hq-mobile`: `Key::Left`, `Key::Right`, and `Phone::move_cursor_to(row, col)`, which sends the
  arrow presses from the cursor to a tapped cell on its row (D2).
- `android-app` spec: "Session screen without a text box" (the keys) and "Typing into the agent's
  box" (a tap moves the cursor) change; "Moving the cursor" is new.

## Non-goals

- Moving between lines of a message by tapping (↑ and ↓ do it, as on the desktop).
- Selecting text in the agent's box.
- The keyboard's own cursor gestures (the space-bar swipe).

## Capabilities

No new project capability. Modified: `android-app`. Verified against Claude Code 2.1.319 and Codex
0.166.0; agy unchecked.

## Impact

`hq-mobile` (`terminal.rs`, `lib.rs`), Android `ui/SessionScreen.kt`. No protocol change: arrows
are input bytes. Delivered in the same pull request as `add-image-upload`, since both change the
key row.
