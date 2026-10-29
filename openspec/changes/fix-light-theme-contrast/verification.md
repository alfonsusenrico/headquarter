# Verification

Working tree on `fix/light-theme-contrast` (rebased onto 0d4e1ee), 2026-10-29, macOS.

## Diagnosis

`research/evidence/fix-light-theme-contrast/findings.md`, with the probe's raw reports and Claude Code
2.1.312's theme tables beside it. Main cause: Claude Code draws its default dark theme in a light
terminal; then the light templates' chrome (flat single surface, muted and needs-you under AA) and
their palettes.

## Automated

- `./scripts/check`: exit 0. 552 Rust tests passed, 0 failed (1 ignored, as before); rustfmt, clippy
  `-D warnings`, cargo-deny; OpenSpec valid (strict); Android ktlint, Lint, unit tests and builds.
- New tests:
  - `hq-app` `light_themes_are_designed_for_light`: every shipped light theme (Light, Catppuccin
    Latte, Solarized Light, Gruvbox Light) keeps text, muted, group, needs-you and accent at 4.5:1
    and the needs-you glyph at 3:1 on the sidebar, the selected row and the terminal; the sidebar
    sits darker than the terminal; dark templates keep the chrome they had.
  - `hq-app` `readable_text_meets_the_minimum_with_the_smaller_step` and
    `text_under_the_minimum_contrast_is_made_readable`: Claude Code's dark amber and its light
    grey reach 4.5:1 on white in the same hue, dark text lightens on a dark background, hidden text
    stays hidden, 1 leaves colours as sent.
  - `hq-signals` `the_theme_follows_the_terminal_unless_the_owner_chose_one`: `light` or `dark` is
    passed with the hooks; a plain `dark` set in Claude Code follows the terminal; a
    `dark-daltonized` one is kept; nothing is passed before the app has said.
  - `hq-signals` `the_background_is_kept_whole`, `hq-config`
    `the_minimum_contrast_is_a_window_setting` and the range check.
- Frame build, 120x40 screen (release): 94.2 µs with the minimum at 4.5, 92.1 µs with it off.

## On screen

The dev app with a synthetic `HOME`, and a stand-in Claude Code
(`research/prototypes/agent-colours/claude-standin.py`) that draws a typical screen (welcome box,
edit with diff lines, cargo output in the palette's colours, permission box, prompt, status line) in
Claude Code 2.1.312's own light or dark colours, taking only the theme Headquarter passes.

- Before (minimum off, Claude dark), Catppuccin Latte: the owner's finding reproduced (invisible
  white bullets, faint lavender and amber, dark text on dark diff bars, one flat surface).
- The owner compared four states and chose "apart".
- After, Catppuccin Latte, Solarized Light, Gruvbox Light and built-in Light: `terminal-background`
  says `light`, the host passed `"theme":"light"` in Claude Code's `--settings`, the stand-in drew its
  light theme, every line readable; the template sidebars sit a step darker than the terminal.
- Built-in Dark: `terminal-background` says `dark`, Claude Code gets `"theme":"dark"`, the look is as
  before.
- The real Claude Code 2.1.312 with the owner's own settings (`"theme": "dark"`) and
  `--settings '{"theme":"light"}'` drew its light colours (probe), so the per-launch theme wins over
  the settings file.

## Not verified

- The real Claude Code inside the app (the stand-in uses its colours; the probe shows the real CLI
  honours the passed theme).
- A session the host restarts at its own start (same launch code path, not seen on screen).
- agy's session text in a light theme; Linux.
- The owner's own theme file keeps the colours it sets (by design); its terminal gains Claude's
  light theme and the minimum contrast.
