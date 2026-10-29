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

## Diagnosis results (2026-10-29)

Full record: `research/evidence/fix-light-theme-contrast/findings.md`.

- Main cause: Claude Code 2.1.312 draws its default `dark` theme in a light terminal (amber 1.6:1,
  lavender 1.9:1, grey 2.9:1 on white; white bullets and dark diff bars). It asks the terminal's
  background only with its `auto` theme, and a session the host starts has no window to answer.
  `theme` is a key of its settings schema, so `--settings` sets it per launch.
- Codex 0.164.0 asks OSC 10 and 11 and adapts. agy asks OSC 11; its text in a session is still to
  be seen.
- The built-in Light chrome passes. The three light templates (Latte, Solarized Light, Gruvbox Light)
  make their chrome from their terminal colours: muted 2.4 to 4.2:1, needs-you 2.2 to 3.0:1, and the
  sidebar exactly the terminal's background, so the window is one flat colour. Several of their
  terminal colours are under 3:1. The owner's own theme, made from Latte, has the same pattern.
- The owner compared four states on screen (as today, minimum contrast only, "apart", and a dark
  terminal in light chrome) and chose "apart" (owner, 2026-10-29).

Owner's guidance (2026-10-29): a light theme does not paint every background in the one light
colour; the terminal must above all stay highly readable. The target is the themes Headquarter
ships.

## Decisions

### D1. Light templates designed for light ("apart")

A light theme made from terminal colours (the templates, and Alacritty colours) gets its chrome
designed for light: the sidebar a step darker than the terminal (6 % toward the text), the header
staying with the pane above its divider,
the selected row in the terminal's background, dividers at 20 %, and muted, group, needs-you text
and accent made readable (4.5:1) against both the sidebar and the terminal background, the
needs-you glyph at 3:1. The terminal keeps the theme's own background. Interface colours a theme
file sets itself still win, so the owner's own file keeps its choices. Dark themes are unchanged.

Rejected: a dark terminal inside light chrome (owner, 2026-10-29: compared on screen).

### D2. Claude Code told light or dark at launch

The app writes whether its terminal is light or dark to `terminal-background` in the data folder
whenever the theme changes. The host reads it at each Claude Code launch and adds `"theme": "light"`
or `"dark"` to the `--settings` it already passes. A theme the owner set in Claude Code itself
(`theme` in its settings or global config, read only) is kept when it is a colour-blind, ANSI or
custom theme; a plain `light`, `dark` or `auto` follows the terminal (owner, 2026-10-29: the owner's
own settings say `dark`, which Claude Code writes from its first-run theme screen). Never by editing
the agent's files.

Rejected: Claude's `auto` (no window answers it for a session the host starts, and its first frame
is drawn dark); a protocol request (a protocol change would force a phone update for a desktop
detail).

### D3. A minimum contrast in the terminal

Text under the minimum against its own cell background is moved toward black or white, whichever
needs the smaller step, until it meets it; hidden text is left alone. Default 4.5:1 in light and
dark themes (owner, 2026-10-29), setting `window.minimum_contrast` (1 turns it off, up to 21).
Results are kept per colour pair for the frame, since a screen drawn for the other mode repeats a
few pairs in thousands of cells. Rejected: remapping agents' colours to the palette (it breaks
colours they chose on purpose).

### D4. Marks keep their colours

The agents' marks are the vendors' brand marks, drawn from their own image files, and keep their
colours in both modes (the framework's rule: brand marks keep their colours). Claude's orange
measures 2.5 to 2.7:1 on the light sidebars; it is recorded, not shifted, because changing a vendor's
logo is what the owner first reported as wrong. The one-colour marks follow the text colour and the
drawn fallback the muted colour, both readable.

## Risks

- A theme an owner made by hand may set chrome colours that fail. The checks report them, and the
  app does not override the owner's choices.
- Minimum contrast changes colours an agent chose. It only moves lightness, only below the
  threshold, and can be turned off.
