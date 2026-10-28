# Verification: sessions already in a folder

Tested: branch `feat/folder-sessions` @ e911235 from `main` @ 430cf19. 2026-10-28.

## Commands

- `./scripts/check`: passed. Rust 538 tests in 27 suites, 0 failed. fmt, clippy `-D warnings`,
  cargo-deny (advisories, bans, licenses, sources ok). `openspec validate --all --strict`:
  19 items, 0 failed.
- The change's own tests, all passing in that run:
  - `folders_count_sidebar_sessions_and_past_conversations`: sidebar and past together, outside
    sessions, ones running in another terminal and ones not offered left out.
  - `only_sessions_not_in_the_sidebar_are_offered`: all in the sidebar gives none, one of two
    not added gives that one, neither added gives both newest first, a not-asked one gives none.
  - `the_not_asked_list_grows_and_forgets_sessions_that_are_gone`,
    `sessions_not_to_ask_about_are_kept_with_the_rest`.
  - `folder_row_labels_meet_wcag_aa` (D5): muted on a hovered row, the group colour on the
    highlighted row, hovered or not, in light and dark.

## On screen

macOS 27.0.1, dev build of e911235. A synthetic home folder (`HOME`, `HQ_DATA_DIR`,
`HQ_CONFIG_DIR` under a temporary folder): stand-in agents in `~/code/api-server`,
`~/code/web-app` and `~/code/notes`, and three made-up Claude Code transcripts, two in web-app
(2h and 3d old) and one in api-server (1d old). No real folders or conversations are on screen.
Window captures: `research/evidence/folder-sessions/`.

- Counts in `~/code/`: api-server "2 sessions", notes "1 session", web-app "3 sessions",
  website none. On the highlighted row the count, "Open here" and › use the group colour.
- Start in web-app: "Not in your sidebar", `~/code/web-app`, the two past conversations newest
  first with "Claude Code · last used 2h ago" and "… 3d ago". Esc returned to the window.
- Continue on "Fix the login redirect loop": both windows closed, it joined the sidebar and
  resumed (the stand-in printed its ready line).
- Start in web-app again: only "Add a dark mode toggle" offered. Ticked "Don't ask again for
  these", Enter started a new session. `app-state.json` kept its id under `not_asked`.
- After quitting (Keep Running) and reopening the app: web-app read "4 sessions" and started
  without a question.
- Light theme: api-server offered "Rate limit the upload endpoint", Cancel returned to the
  window. Notes, whose only session is in the sidebar, started without a question.

Found while checking: midway, the owner's own Headquarter became frontmost and received a ⌘N
and a typed path meant for the dev app. Its New session window showed "No folder", nothing was
started, and Esc closed it. The driving script now checks the frontmost app before every key.

## Not verified

- Linux.
- Codex and agy past conversations (only Claude Code transcripts were made up). They reach the
  app through the same session list.
- A folder reached through a symlink (design risk: counted under the path used).
- A failed Continue (the host error goes to the window's footer, by code review only).

## Owner check

Pending.
