# Changelog

All notable changes to Headquarter. Versions follow [Semantic Versioning](https://semver.org).

## 0.1.6 (2026-10-24)

- Your clipboard stays yours. Opening a session no longer swaps it for that session's
  old selection: Claude Code re-sends its last selection whenever it redraws, and
  Headquarter now only takes an agent's copy right after you dragged, clicked or typed
  in that session.

## 0.1.5 (2026-10-24)

- Notifications say which session they're about: the title is what the sidebar shows,
  such as "Fixing Unresponsive Docker Daemon", instead of an id like "claude-e0c6f139".
- Usage details keep every window name on one line, including agy's per-model windows
  ("week · Gemini") next to "reset since this reading".

## 0.1.4 (2026-10-23)

- Select and copy like you mean it. Drag over any agent's output and it lands on your
  clipboard when you let go, with a "Copied" note to say so. Double-click takes a word,
  triple-click a line. Copies come out tidy: no trailing spaces, no shared left margin,
  no blank lines at the ends.
- Claude Code's full-screen view gets the drag itself, so it scrolls through its own
  history and copies. Copies an agent makes now reach your clipboard. Hold Shift to
  select in Headquarter instead.
- Dragging keeps going outside the pane and scrolls back through the output past the
  top or bottom. A drag back to the left edge now takes the first column too.
- Selected text stays readable: themes that set a selection text colour, like
  Catppuccin, now get it.

## 0.1.3 (2026-10-23)

- Drag a screenshot onto a session and the agent gets it: dropped files are typed as
  their paths, escaped like Terminal.app does, so Claude Code attaches images just as it
  does in Alacritty.
- Groups have colours. Each header gets a dot and a stronger title, and its sessions sit
  beside a thin line of the same colour. New groups pick a soft colour on their own;
  click the dot to choose another.
- A session still running in another terminal shows just "ext" and its age. Its status,
  "Needs you" and notifications come back once it runs in Headquarter.
- Settings copied from Alacritty now look like Alacritty: a font style the font doesn't
  have (such as bold "Heavy" in JetBrains Mono) falls back to your normal face instead
  of the nearest heavy one, and lines are spaced by the font itself.

## 0.1.2 (2026-10-23)

- Add sessions: a session still running in another terminal can now be added. It
  shows as "ext" with its live status; close it there and the pane says "Closed in the
  other terminal." with an Open here button, so you continue it in Headquarter without
  a round trip. Headquarter still never touches the other terminal's process.
- Add sessions keeps up with agents in other terminals: no more greyed "claude pid N"
  rows next to the same session, and a newly started one appears without reopening the
  app. Tags and ages line up.
- Linux: the running window shows the Headquarter icon in the dock, taskbar and window
  switcher.

## 0.1.1 (2026-10-23)

- First run: setup lists only the agents Headquarter hosts, with their versions and
  install links, lets you pick a theme with a live preview, and ends with the three
  shortcuts you need.
- While the host starts or sessions load, the agents' marks light up in turn instead of
  an empty window.
- Usage: each agent's ring now shows both windows, 5 hours inside and the week outside,
  and turns red from 80%. The line under the rings names the one agent near its limit,
  or says how many are. Refresh keeps the details open until the new numbers arrive.
- agy's usage, read with agy's own `/usage`, one window per pool of models.
- Add sessions:
  - sessions running in another terminal are tagged and cannot be added;
  - it offers only conversations you started that the agent still keeps, and whose
    folder still exists;
  - untitled conversations read "Untitled conversation" with their short id;
  - a ⋯ menu removes a past conversation: Claude Code's to the Trash, Codex's with its
    own archive or delete, with Undo.
- Fixes: agents start with their own runtime on `PATH` (Codex from Homebrew); the folder
  search no longer makes macOS ask for Music or Photos; idle sessions in other terminals
  are flagged at once; past conversations show their titles; an agy session in another
  terminal no longer shows twice; a closed session no longer comes back after a restart;
  usage problems are worded from the agent's side.

## 0.1.0 (2026-10-22)

The first release.

- Host every Claude Code, Codex and Antigravity (`agy`) session in one app. Sessions
  keep running when the window closes, and `hq` drives the same host from any terminal.
- Status for every session from the agents' own reports: working, needs you, done but
  not read, idle and not running.
- A sidebar with your own groups, a "Needs you" section, an icon rail, and keyboard
  navigation.
- Agents running in other terminals, listed beside your own sessions (read-only).
- Past conversations imported, so you can open them again.
- Usage for Claude Code and Codex in the footer, with reset times.
- Session restore: the window, the session you were viewing, and a prompt to resume
  sessions a crash interrupted.
- Themes, including your Alacritty settings and colours.
- A one-line installer for macOS and Linux, a macOS app with Headquarter, Edit and
  Window menus, and an app icon.
