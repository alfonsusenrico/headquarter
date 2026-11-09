# Verification: refine-phone-typing-and-links

Revision: `feat/phone-typing-and-links` at a55ae12 (code), on `main` 4deda3d. Not yet run on a
phone: the on-phone checks (tasks 2.3, 4.2) wait for the owner's go.

## Automated (2026-11-09, macOS, Rust from `rust-toolchain.toml`, JDK 17)

| Check | Result |
| --- | --- |
| `./scripts/check` (fmt, clippy `-D warnings`, tests, cargo-deny, OpenSpec) | passed: 614 tests, OpenSpec 19/19 |
| `hq-links` (the desktop's link tests, moved) | 6 passed |
| `hq-mobile` core: `a_tapped_cell_gives_its_web_link`, `a_link_is_found_where_it_is_drawn_while_scrolled_back`, `an_edit_erases_then_types_without_enter` | passed |
| Host test (`hq-cli/tests/host.rs`): the phone core types `from-the-coer`, erases 2, types `re`, sends Enter through a real host | the stub read `from-the-core` |
| `./scripts/android-core` | built, binding has `edit` and `linkAt`, no `typeText` |
| `./gradlew ktlintCheck assembleDebug lint testDebugUnitTest` | passed: 15 JVM tests (7 in `LiveTypingTest`), Lint with warnings as errors clean |

Agents: Claude Code 2.1.318 and Codex 0.166.0 take Ctrl+J as a new line
(`research/evidence/phone-typing-images/2026-11-06-agent-input.md`). agy 1.5.1 not checked;
its sessions keep "Enter sends".

## Not verified yet

- What Gboard sends for Backspace on an empty field (task 2.3); the build uses the key event.
- Every spec scenario on the owner's phone (task 4.2): typing, a corrected word, swipe and voice
  typing, erasing recalled text, a two-line message, a shell command, opening and copying links,
  "No app can open this link".
