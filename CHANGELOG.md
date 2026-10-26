# Changelog

All notable changes to Headquarter. Versions follow [Semantic Versioning](https://semver.org).

## 0.1.12 (2026-10-26)

- Links open with a click. Hover a web address in an agent or the shell panel and it
  underlines; click and it opens in your browser, even when the agent wrapped it over
  several lines. Only http and https links open.
- Copying a wrapped line from a long Claude Code session now gives you one clean
  line. After a session had printed a lot, Headquarter forgot it was in fullscreen, so
  drags, clicks, the mouse wheel and multi-line pastes stopped going to Claude Code.
- Working in a session's shell panel counts as watching it: no unread flag and no
  notification for the session you are in.
- Restart with the Update pill: the new host brings the session fixes, and your
  sessions start again on their own.

## 0.1.11 (2026-10-26)

- Shift+Tab now reaches your agent, so you can cycle Claude Code between accept edits,
  plan mode and default. Until now it jumped around the sessions list instead. Elsewhere in
  the window, Tab still moves between controls.

## 0.1.10 (2026-10-24)

- Restarting for an update no longer ends with "cannot reach the Headquarter host". The
  old background host takes a moment to end your agents before it exits, and the new
  one now waits for it instead of giving up.
- Coming from 0.1.9: use the Update pill. The restart that brings you to 0.1.10 already
  starts the new host, which carries this fix.

## 0.1.9 (2026-10-24)

- Quit to save memory, not your place. `/exit` an agent and its memory goes back to your
  Mac (a Claude Code session holds 270-720 MB), but the session stays in the sidebar,
  dimmed with a moon. Open it and press Resume Session to carry on where it was. The ✕
  still closes a session for good, and asks before removing an inactive one.
- Sessions come back by themselves. After a reboot, a crash or `hq stop`, whatever was
  running starts again on its conversation, the session you were looking at first, the
  rest one at a time while they breathe "starting". Inactive sessions stay asleep.
  The "N sessions were running" note and `hq resume --interrupted` are gone.
- A session that can't start (its folder was deleted, say) stays inactive, says why,
  and tells you once with a notification. With its folder gone, the pane offers Remove
  from Sidebar instead of a Resume that could only fail.
- A Claude Code session you opened but never typed in now resumes as the empty
  conversation it was, instead of failing.
- Updates from inside the app. A new release shows as an "Update" pill at the bottom of
  the sidebar (or choose Headquarter > Check for Updates…). One click downloads it,
  checks the installer against the checksum GitHub publishes, installs it and restarts
  Headquarter, asking first if a session is busy, and your running sessions come back.
  `[updates] check = false` turns the automatic checks off.
- `hq ls` says "inactive" and "starting".
- Coming from 0.1.8: install this one with the install line, since 0.1.8 cannot update
  itself. Then open Headquarter and press "Restart to finish update" in the sidebar:
  the old background host stops, and the sessions it ran start again on the new one.

## 0.1.8 (2026-10-24)

- A shell beside every agent. Cmd-T (Ctrl-Shift-T on Linux) opens a terminal under the
  session, in its folder, without touching the sidebar. Drag the line between them to
  share the room. The shell keeps running while you look at other sessions, and ends
  when you press Cmd-T again, type `exit`, or close the session. Plain Ctrl-T still goes
  to the agent.
- The shell runs in Headquarter's background host, which an update doesn't restart. If
  your host was already running before the update, the panel reports an error until
  the host restarts (`hq stop`, which also ends your sessions, then reopen the app).

## 0.1.7 (2026-10-24)

- A Claude Code session running a background command shows "working" again until the
  command finishes. Claude Code 2.1.310 started calling that state "shell", which
  Headquarter didn't recognise, so the session looked idle.

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
