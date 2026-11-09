# Type into the agent's own box on the phone, and tap links

Status: approved by the owner 2026-11-09 ("approve, build it").

## Why

The owner (2026-11-06), using the phone app daily:

- The session screen has its own text box under the terminal while the agent draws its own
  input box in the terminal. The owner taps the agent's box by mistake, and nothing happens: the
  terminal ignores taps (`ui/TerminalView.kt` handles pinch and drag only).
- The key row's ⏎ and the keyboard's Enter both send, so there is no way to write a message of
  several lines.
- Links in the terminal cannot be tapped. The desktop already opens them
  (`crates/hq-app/src/terminal/links.rs`, `terminal-pane` "Links").

Decisions: `research/notes/2026-11-06-phone-typing-images-links.md`.

## Expected Outcome

- Tapping the terminal opens the keyboard, and what the owner types appears in the agent's own
  input box as it is typed. Autocorrect, swipe typing and voice typing keep working. The app's
  text box is gone; the key row stays.
- The keyboard's Enter starts a new line in the agent's box; the key row's ⏎ sends. In the shell
  the keyboard's Enter runs the command.
- Tapping a web link opens it in the browser; a long press copies it.

## What Changes

- The session and shell screens lose the text box. A hidden field takes the keyboard's input and
  sends each edit to the session as it happens (D1, D2).
- Enter from the keyboard sends a new line (Ctrl+J) to an agent and Enter to a shell (D3).
- The phone core finds the link at a tapped cell with the desktop's link finder, moved to a crate
  both use (D4, D5).
- `android-app` spec: "Session on the phone" changes; "Session screen layout" is replaced by
  "Session screen without a text box"; "Typing into the agent's box", "New line and send" and
  "Links on the phone" are new.

## Non-goals

- Sending images (the next change, `add-image-upload`).
- Moving the agent's cursor from the phone (the keyboard's cursor gestures do nothing).
- Several computers in one list on the phone (after 0.3.0).
- Underlining links before they are tapped.

## Capabilities

No new project capability. Modified: `android-app`. Verified against Claude Code 2.1.318 and
Codex 0.166.0 (Ctrl+J is a new line in both, `research/evidence/phone-typing-images/`). agy 1.5.1
is not checked yet: it needs a folder it trusts.

## Impact

- Android: `ui/SessionScreen.kt` (no `Composer`), `ui/TerminalView.kt` (tap, long press, the
  hidden field), a new `ui/LiveTyping.kt`.
- `hq-mobile`: `type_text` replaced by `edit` (erase, then text, no Enter), `link_at`.
- New crate `hq-links` with the link finder from `hq-app/src/terminal/links.rs`; `hq-app` uses it
  unchanged.
- No protocol change: edits are input bytes, as typing is today.
