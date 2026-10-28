# Light themes that read as light

Status: proposed 2026-10-28, waiting for the owner's approval. The owner asked for plans only:
no implementation until approved, after a few days of using 0.1.17.

## Why

The owner's finding (2026-10-28, screenshot): every light theme, built-in or made in the theme
creator, looks like a washed-out negative of the dark theme rather than a light theme. Sidebar,
terminal and header turn grey, terminal colours such as the light theme's blue become hard to
read, and the agents' marks change colour.

Observed on the owner's machine: the settings file sets `[window] opacity = 0.7` and `blur = true`
(carried over from Alacritty, tuned for a dark theme), over a dark desktop picture. The cause is
not confirmed yet. Two explanations fit (design, Diagnosis):

- H1: in a light theme the window's white surfaces are drawn at 70% over the blurred dark desktop,
  so white becomes mid grey and every colour meant for white loses contrast.
- H2: some colours do not follow the chosen light theme (for example the imported Catppuccin
  Mocha colours, or marks drawn for the dark mode), and mix with the light ones.

## Expected Outcome

- With any light theme, the app reads as a light theme on any desktop picture: white or near-white
  surfaces, and text and terminal colours at their intended contrast (AA).
- Dark themes keep the owner's translucency and blur exactly as today.
- The agents' marks keep their colours in both modes.
- The settings file can still ask for a translucent light window, deliberately.

## What Changes

Final shape decided after the diagnosis tasks, then confirmed with the owner if it differs from
D2:

- `hq-app` `settings.rs`: the window's opacity applies to dark themes, and light themes draw
  opaque unless the owner sets a light-mode opacity.
- `hq-config`: an optional `[window] opacity_light` (Headquarter's own key beside Alacritty's).
- `settings` spec: the appearance requirement says how opacity applies per mode.

## Non-goals

- The colours of the built-in light theme or of any template.
- Dark themes' look.
- The phone app.

## Capabilities affected

`settings`. No new project capability.

## Impact

Desktop app only. Owners with `opacity` below 1 and a light theme see an opaque window after the
change. Their setting keeps working in dark themes.
