# Light themes that read as light

Status: proposed 2026-10-28, revised the same day after the owner's test; waiting for the owner's
approval. The owner asked for plans only: no implementation until approved, after a few days of
using 0.1.17.

## Why

The owner's findings (2026-10-28, two screenshots):

- Every light theme, built-in or made in the theme creator, looks like a washed-out negative of
  the dark theme rather than a light theme, and some colours (the agents' marks among them)
  change in ways they should not.
- With `opacity = 1` and blur off it looks the same (owner's test), so window translucency is not
  the cause. A theme the owner tuned by hand is readable but still not good: "maybe because it's
  fully light background and then just texts?"
- The owner wants the UI and UX rules updated so a light theme is designed for contrast, not
  derived from the dark one.

What the screenshots show, to be confirmed by the diagnosis:

- Inside the terminal, Claude Code draws with its own colours: light-blue code spans, light grey
  dimmed text, a dark bar behind a past prompt, a dark "Jump to bottom" chip. Those are Claude
  Code's dark-theme colours on a white terminal. The owner's `~/.claude.json` sets no theme, and
  Headquarter answers colour queries (OSC 10, 11) with the light palette, so Claude Code does not
  use them to choose its colours.
- In Headquarter's own chrome, the session header is a grey strip with light grey text (folder,
  model, figures), and the footer's usage rings and marks look dim.
- The light theme is mostly white surfaces with text, with little separation between the
  sidebar, the header and the pane.

## Expected Outcome

- With any light theme, Headquarter's own surfaces, text, icons and marks meet their contrast
  targets (text AA 4.5:1, icons and marks 3:1), and the sidebar, header and pane are told apart
  by surfaces and lines rather than by text alone.
- Agents that choose their own colours draw their light colours in a light theme, where
  Headquarter can say so per launch. Where an agent's colour still lands on the wrong background,
  the terminal keeps its text readable (a minimum contrast, in light and dark themes alike).
- The agents' marks keep their brand colours in both modes.
- Dark themes look as they do today.
- The rule for designing each theme for its own background lives in both places the owner chose
  (2026-10-28): the framework's UI and UX skill (a Pakem proposal) and this project's
  `AGENTS.md` (branch `docs/theme-contrast-rule`).

## What Changes

Final shape decided after the diagnosis tasks:

- `hq-app` `theme.rs` and the chrome that draws the header, footer and marks: light tokens and
  where they are used, with theme tests for every pair.
- `hq-app` terminal: a minimum contrast for text drawn by agents, in light themes at least.
- `hq-signals` adapters: a per-launch light or dark hint for agents that accept one (Claude Code
  first), without touching the agents' own configuration.
- `settings` spec: how light themes, minimum contrast and the agents' hint behave.

## Non-goals

- Window translucency (it applies as today in both modes, the owner's test showed it is not the
  cause).
- Changing the agents' own configuration files.
- The phone app.

## Capabilities affected

`settings`, `terminal-pane`, `agent-status` (launch instrumentation only). No new project
capability.

## Impact

Desktop app and host launch specs. Verified against Claude Code, Codex and agy as installed when
built.

## Owner decisions (2026-10-28)

- The rule goes in both the framework's UI and UX skill and `AGENTS.md`.
- The terminal's minimum contrast applies to dark themes too.
