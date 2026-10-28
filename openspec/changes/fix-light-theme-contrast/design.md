# Design: light themes that read as light

## Context

- `theme.rs` holds the chrome tokens (`Chrome`: sidebar, sidebar_line, selected, text, muted,
  group, amber, glyph_amber, track, accent) and the terminal palette for light and dark, with
  theme tests holding several text pairs to AA. Templates and theme files bring their own colours.
- The session header paints `chrome.sidebar` with a `sidebar_line` border (`workspace/header.rs`).
- The terminal answers OSC 10, 11 and 4 colour queries from the palette (`terminal/model.rs`).
- Claude Code keeps its theme in `~/.claude.json`, which Headquarter never edits. The owner's file
  sets none.
- Owner's test: `opacity = 1`, blur off, the same look. Translucency is ruled out.

## Diagnosis (before any fix)

On the dev app with a test data folder, a built-in light theme and the owner's own light theme,
and stand-in and real agents:

1. Headquarter's chrome: capture the sidebar, header, footer and a modal. Measure every text,
   icon and mark against its own surface, and the separation between surfaces. Trace each
   failing colour to its token or to a hard-coded value.
2. Claude Code: run it in a light terminal with no theme set, then with each of its light themes
   set in a scratch `CLAUDE_CONFIG_DIR`. Record which colours it draws (truecolor or palette),
   whether it reads OSC 11 or `COLORFGBG`, and whether any per-launch flag, `--settings` key or
   environment variable selects its theme. Record the version.
3. Codex and agy: the same, in a light terminal.
4. Measure the contrast of agent text as drawn in 2 and 3 on the light background.

Findings go to `research/evidence/fix-light-theme-contrast/findings.md`.

## Decisions (to confirm after the diagnosis)

### D1. Light tokens designed for light

Each failing chrome pair gets a light token designed for its surface, not the dark token reused.
The header, sidebar and pane are separated by distinct surfaces or lines that meet 3:1 against
each other where they carry meaning. A theme test covers every chrome pair in both modes, and
templates and theme files are checked by the same test where they set chrome colours.

### D2. Agents told the mode, per launch

Where an agent accepts a per-launch theme (a flag, a `--settings` key, an environment variable),
its launch spec passes light or dark to match Headquarter's theme. Where it reads the terminal's
background (OSC 11 or `COLORFGBG`), the terminal answers it truthfully, as today, and Headquarter
sets `COLORFGBG` per launch if the agent reads it. Never by editing the agent's own files.

### D3. A minimum contrast in the terminal

When a cell's foreground and background fall under a minimum contrast (about 3:1, as Ghostty's
`minimum-contrast` and iTerm2's Minimum contrast do), the terminal shifts the foreground's
lightness until it meets it, keeping its hue. On by default in light and dark themes alike (owner,
2026-10-28), with a setting to change the minimum or turn it off. In dark themes it lifts text
an agent drew too dark for the background, the mirror of the light case. Rejected: remapping agents' colours to the palette (it breaks colours they chose
on purpose).

### D4. Marks keep their colours

The agents' marks are drawn in their own colours in both modes, with an outline or a tone shift
only where a mark falls under 3:1 against its surface.

## Risks

- A theme an owner made by hand may set chrome colours that fail. The checks report them, and the
  app does not override the owner's choices.
- Minimum contrast changes colours an agent chose. It only moves lightness, only below the
  threshold, and can be turned off.
