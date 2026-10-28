# Design: light themes that read as light

## Context

- `settings::window_background` turns `[window] opacity` and `blur` into GPUI's Opaque, Blurred or
  Transparent window background, whatever the theme's mode.
- `set_controls_mode` clears the UI kit's root background so the sidebar and the pane paint their
  own backgrounds at the window's opacity.
- `themes::look`: Light uses `Theme::light()`, templates and theme files use their own colours,
  and only the Alacritty choice merges the imported colours (`Theme::resolve`).
- The owner's file: `opacity = 0.7`, `blur = true`, an imported Catppuccin Mocha colour file, and
  a dark desktop picture.

## Diagnosis (before any fix)

H1 and H2 are told apart by changing one thing at a time on the dev app, with a dark desktop
picture behind it and the owner's settings copied into a test folder:

1. Light theme, `opacity = 0.7`, `blur = true`: captures the reported look (reproduction).
2. Same with `opacity = 1`: if the look is right, H1 holds.
3. Same with `opacity = 0.7` over a white desktop picture: if it is right, H1 holds (the desktop
   shows through).
4. `opacity = 1`, the imported colours removed: any colour still wrong (marks, terminal) points
   to H2, and each wrong colour is traced to where it is set.

Contrast of the reported pairs is measured from the captures (text on surface, terminal blue on
background, the marks).

## Decisions (to confirm after the diagnosis)

### D1. Translucency follows the mode

`window_background` gets the theme's mode. In a dark theme it behaves as today. In a light theme
it uses `opacity_light`, which defaults to 1 (opaque), so blur and translucency are off unless the
owner asks for them.

Rejected: one opacity for both modes (the reported problem), and a fixed floor such as 0.92 for
light themes (a dark desktop still tints white grey, and the owner would have no way to opt out).

### D2. `opacity_light`

`[window] opacity_light = 0.0 to 1.0`, Headquarter's own key next to Alacritty's `opacity`,
reported like any other appearance setting when out of range. Absent means 1.

### D3. Marks and other colours (only if H2 shows any)

Each colour that does not follow the light theme is fixed where it is set, with a theme test
checking it in both modes.

## Risks

- An owner who liked a translucent light window sees it turn opaque until they set
  `opacity_light`. The changelog says so.
