# Verification: a cleaner session screen on the phone

Tested: branch `feat/phone-session-refresh` from `main` @ 2efb497, working tree with the change.
2026-10-27.

## Commands

- `./scripts/check`: passed. Rust 504 tests, 0 failed. ktlint, Android Lint (warnings as
  errors), app unit tests 6, debug and release APKs. OpenSpec 19 items, 0 failed.
- `openspec validate refresh-phone-session --strict`: valid.

## On the dev phone

Phone: realme RMX1921, debug build. Host: isolated test host, stand-in agent `echo-agent`
(prints history, then names every key it receives). Screenshots, status bar cropped:
`research/evidence/refresh-phone-session/`.

- Header: back icon, title, status glyph and word, agent and folder, shell icon button. The
  shell's header says "Shell · title" and the folder.
- Keys, keyboard closed: two rows, ⏎ across both in accent. Every key reached the agent: 1, 2,
  3, Esc, Shift-Tab, Up, Down, Tab, Ctrl-C, Enter.
- Keys, keyboard open: one scrolling row with ⏎ pinned. They return to the grid when the
  keyboard closes.
- Text box: no Send while empty, Send inside with text, four lines then scrolling. The Send
  button and Gboard's Send key both typed the text, then Enter.
- Live: shows when scrolled back, returns to the newest output and hides.
- Shell: its six keys and ⏎ in one row. Found and fixed: "Ctrl-C" and "Ctrl-D" were cut off at
  13 sp, the labels now shrink to fit. A command went through exactly as typed, Ctrl-C
  interrupted the prompt.
- Dark and light themes.

## Not verified

- A long session title on screen (the stand-in's title is short). It uses the same one-line
  ellipsis as before.
- A hardware keyboard's Enter adds a line in the text box, as the old box did.
- Other phones and font sizes.
