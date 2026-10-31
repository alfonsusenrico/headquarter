//! Colour tokens: the window chrome and the terminal palette, for the built-in light
//! and dark themes. This is the project's tokens file.

use alacritty_terminal::vte::ansi::Rgb;
use hq_config::ColorSettings;
use hq_protocol::GroupColor;

pub const fn rgb(hex: u32) -> Rgb {
    Rgb {
        r: ((hex >> 16) & 0xFF) as u8,
        g: ((hex >> 8) & 0xFF) as u8,
        b: (hex & 0xFF) as u8,
    }
}

/// A colour with its opacity in percent. Interface and terminal colours carry one, so
/// a theme can make any of them see-through (add-theme-creator D4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Paint {
    pub rgb: Rgb,
    pub opacity: u8,
}

impl Paint {
    pub const fn solid(rgb: Rgb) -> Paint {
        Paint { rgb, opacity: 100 }
    }

    pub const fn with_opacity(self, opacity: u8) -> Paint {
        Paint {
            rgb: self.rgb,
            opacity: if opacity > 100 { 100 } else { opacity },
        }
    }

    pub fn alpha(self) -> f32 {
        f32::from(self.opacity) / 100.0
    }
}

impl From<Rgb> for Paint {
    fn from(rgb: Rgb) -> Paint {
        Paint::solid(rgb)
    }
}

pub const fn paint(hex: u32) -> Paint {
    Paint::solid(rgb(hex))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Light,
    Dark,
}

/// Window colours outside the terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chrome {
    pub sidebar: Paint,
    pub sidebar_line: Paint,
    pub selected: Paint,
    pub text: Paint,
    pub muted: Paint,
    /// Folder headers: between text and muted.
    pub group: Paint,
    /// Amber text, such as "N needs you".
    pub amber: Paint,
    /// The "needs you" glyph; a graphic, so 3:1 against the sidebar suffices.
    pub glyph_amber: Paint,
    /// Spinner tracks, tag borders and dividers.
    pub track: Paint,
    /// The dot that marks a reply not read yet.
    pub accent: Paint,
    /// "background" and its dots: work still running behind the prompt.
    pub teal: Paint,
    /// "review" and its diamond: a plan waiting for the owner's review.
    pub violet: Paint,
}

/// The colours that mark states, as a palette gives them, before they are made readable
/// on the interface they sit on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hues {
    pub accent: Paint,
    pub needs_you: Paint,
    pub teal: Paint,
    pub violet: Paint,
}

/// The terminal's colours. Agents can override any of them at run time (OSC 4/10/11);
/// those overrides live in the terminal model, not here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub foreground: Rgb,
    pub background: Rgb,
    pub cursor: Rgb,
    pub selection: Rgb,
    /// Selected text; `None` keeps each cell's colour.
    pub selection_text: Option<Rgb>,
    /// ANSI 0–15: black, red, green, yellow, blue, magenta, cyan, white, then bright.
    pub ansi: [Rgb; 16],
    /// Faint versions of ANSI 0–7; `None` blends the normal colour toward the background.
    pub dim: [Option<Rgb>; 8],
    /// Each colour's opacity in percent.
    pub opacity: PaletteOpacity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaletteOpacity {
    pub foreground: u8,
    pub background: u8,
    pub cursor: u8,
    pub selection: u8,
    pub ansi: [u8; 16],
}

impl Default for PaletteOpacity {
    fn default() -> PaletteOpacity {
        PaletteOpacity {
            foreground: 100,
            background: 100,
            cursor: 100,
            selection: 100,
            ansi: [100; 16],
        }
    }
}

impl Palette {
    /// A palette colour by alacritty's numbering (0–15 ANSI, 256 foreground, 257
    /// background, 258 cursor) with its opacity; other numbers are fully opaque.
    pub fn opacity_of(&self, index: usize) -> u8 {
        match index {
            0..=15 => self.opacity.ansi[index],
            256 => self.opacity.foreground,
            257 => self.opacity.background,
            258 => self.opacity.cursor,
            _ => 100,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub mode: Mode,
    pub chrome: Chrome,
    pub palette: Palette,
}

impl Theme {
    /// A usage window near its limit: the theme's terminal red, so it reads as red in
    /// every theme and stays apart from the amber of "needs you".
    pub fn near_limit(&self) -> Paint {
        Paint::solid(self.palette.ansi[1])
    }

    /// `near_limit` while its reading is old: a paler red, the red mixed into the sidebar
    /// rather than see-through, then stepped just enough to keep 3:1 against the sidebar
    /// and the ring's track.
    pub fn old_near_limit(&self) -> Paint {
        let pale = mix(self.chrome.sidebar.rgb, self.palette.ansi[1], 0.7);
        let on_track = readable(pale, self.chrome.track.rgb, 3.0);
        Paint::solid(readable(on_track, self.chrome.sidebar.rgb, 3.0))
    }

    /// A group's colour, from a soft palette per mode whatever the theme
    /// (add-group-colors D1): Catppuccin Mocha in dark mode; in light mode Latte, with
    /// the lighter hues deepened so a dot keeps 3:1 against the sidebar.
    pub fn group_color(&self, color: GroupColor) -> Paint {
        let hex = match (self.mode, color) {
            (Mode::Dark, GroupColor::Mauve) => 0xCBA6F7,
            (Mode::Dark, GroupColor::Green) => 0xA6E3A1,
            (Mode::Dark, GroupColor::Teal) => 0x94E2D5,
            (Mode::Dark, GroupColor::Peach) => 0xFAB387,
            (Mode::Dark, GroupColor::Blue) => 0x89B4FA,
            (Mode::Dark, GroupColor::Pink) => 0xF5C2E7,
            (Mode::Dark, GroupColor::Yellow) => 0xF9E2AF,
            (Mode::Dark, GroupColor::Red) => 0xF38BA8,
            (Mode::Light, GroupColor::Mauve) => 0x8839EF,
            (Mode::Light, GroupColor::Green) => 0x2E8B2B,
            (Mode::Light, GroupColor::Teal) => 0x13868A,
            (Mode::Light, GroupColor::Peach) => 0xC8580F,
            (Mode::Light, GroupColor::Blue) => 0x1E66F5,
            (Mode::Light, GroupColor::Pink) => 0xB8458F,
            (Mode::Light, GroupColor::Yellow) => 0x9A6A0C,
            (Mode::Light, GroupColor::Red) => 0xD20F39,
        };
        paint(hex)
    }

    pub fn light() -> Theme {
        Theme {
            mode: Mode::Light,
            chrome: Chrome {
                sidebar: paint(0xECF0F7),
                sidebar_line: paint(0xD5DBE6),
                selected: paint(0xFFFFFF),
                text: paint(0x1D2433),
                muted: paint(0x5B6475),
                group: paint(0x4A5468),
                amber: paint(0x9A4A06),
                glyph_amber: paint(0xC26A10),
                track: paint(0xC9CFDA),
                accent: paint(0x2457C5),
                teal: paint(0x0E6A5A),
                violet: paint(0x6B3FC2),
            },
            palette: Palette {
                foreground: rgb(0x1D2433),
                background: rgb(0xFFFFFF),
                cursor: rgb(0x1D2433),
                selection: rgb(0xCFDDF7),
                selection_text: None,
                // Every colour but white (7, 15), which agents draw as backgrounds,
                // reads at 4.5:1 or better on the white background.
                ansi: [
                    rgb(0x1D2433),
                    rgb(0xB42318),
                    rgb(0x15703D),
                    rgb(0x855400),
                    rgb(0x2457C5),
                    rgb(0x8E3EA8),
                    rgb(0x0E6A80),
                    rgb(0xE4E8EF),
                    rgb(0x4A5468),
                    rgb(0xC0261B),
                    rgb(0x167A43),
                    rgb(0x7A4D00),
                    rgb(0x1F4FB8),
                    rgb(0x9B3FB8),
                    rgb(0x0B6377),
                    rgb(0xF4F6FA),
                ],
                dim: [None; 8],
                opacity: PaletteOpacity::default(),
            },
        }
    }

    pub fn dark() -> Theme {
        Theme {
            mode: Mode::Dark,
            chrome: Chrome {
                sidebar: paint(0x171A21),
                sidebar_line: paint(0x262B35),
                selected: paint(0x252A34),
                text: paint(0xE6E8EC),
                muted: paint(0x9AA3B2),
                group: paint(0xB4BCC9),
                amber: paint(0xF5A524),
                glyph_amber: paint(0xF5A524),
                track: paint(0x3A404C),
                accent: paint(0x8AB4FF),
                teal: paint(0x67E8C9),
                violet: paint(0xC4A7FF),
            },
            palette: Palette {
                foreground: rgb(0xE6E8EC),
                background: rgb(0x0F1115),
                cursor: rgb(0xE6E8EC),
                selection: rgb(0x2B3A57),
                selection_text: None,
                // Every colour but black (0), which agents draw as backgrounds, reads at
                // 4.5:1 or better on the dark background.
                ansi: [
                    rgb(0x2A2F3A),
                    rgb(0xF87171),
                    rgb(0x4ADE80),
                    rgb(0xFACC15),
                    rgb(0x8AB4FF),
                    rgb(0xD8B4FE),
                    rgb(0x67E8F9),
                    rgb(0xD5DAE3),
                    rgb(0x8A93A3),
                    rgb(0xFCA5A5),
                    rgb(0x86EFAC),
                    rgb(0xFDE68A),
                    rgb(0xB4CDFF),
                    rgb(0xE9D5FF),
                    rgb(0xA5F3FC),
                    rgb(0xF5F7FA),
                ],
                dim: [None; 8],
                opacity: PaletteOpacity::default(),
            },
        }
    }

    pub fn for_mode(mode: Mode) -> Theme {
        match mode {
            Mode::Light => Theme::light(),
            Mode::Dark => Theme::dark(),
        }
    }

    /// The built-in theme for the system's `mode` with the owner's colours on top.
    /// When the owner sets a background, the window chrome is made from their colours
    /// too, and the mode follows that background rather than the system.
    pub fn resolve(system: Mode, colors: &ColorSettings) -> Theme {
        let mode = match colors.background {
            Some(background) if luminance(rgb(background)) < 0.18 => Mode::Dark,
            Some(_) => Mode::Light,
            None => system,
        };
        let mut theme = Theme::for_mode(mode);
        let palette = &mut theme.palette;
        let set = |slot: &mut Rgb, color: Option<u32>| {
            if let Some(color) = color {
                *slot = rgb(color);
            }
        };
        set(&mut palette.foreground, colors.foreground);
        set(&mut palette.background, colors.background);
        set(&mut palette.cursor, colors.cursor);
        set(&mut palette.selection, colors.selection);
        if let Some(text) = colors.selection_text {
            palette.selection_text = Some(rgb(text));
        }
        for index in 0..8 {
            set(&mut palette.ansi[index], colors.normal[index]);
            set(&mut palette.ansi[index + 8], colors.bright[index]);
            if let Some(dim) = colors.dim[index] {
                palette.dim[index] = Some(rgb(dim));
            }
        }
        if colors.background.is_some() {
            let (front, back) = (palette.foreground, palette.background);
            let hues = Hues::from_palette(&theme.chrome, colors);
            theme.chrome = match mode {
                Mode::Light => Chrome::for_light(back, front, hues),
                Mode::Dark => Chrome::from_colours(back, front, hues),
            };
        }
        theme
    }
}

impl Hues {
    /// A palette's blue, yellow, cyan and magenta, or `fallback`'s colours where it has
    /// none.
    pub fn from_palette(fallback: &Chrome, colors: &ColorSettings) -> Hues {
        Hues {
            accent: colors.normal[4].map_or(fallback.accent, paint),
            needs_you: colors.normal[3].map_or(fallback.amber, paint),
            teal: colors.normal[6].map_or(fallback.teal, paint),
            violet: colors.normal[5].map_or(fallback.violet, paint),
        }
    }
}

impl Chrome {
    /// Interface colours made from a background and a text colour, with the given hues:
    /// the mixing amounts Alacritty colours have always used. Teal and violet came later
    /// and are made readable on the sidebar and the selected row
    /// (add-background-work-status D8).
    pub fn from_colours(back: Rgb, front: Rgb, hues: Hues) -> Chrome {
        let selected = mix(back, front, 0.1);
        let on_both =
            |color: Paint| Paint::solid(readable(readable(color.rgb, back, 4.5), selected, 4.5));
        Chrome {
            sidebar: Paint::solid(back),
            sidebar_line: Paint::solid(mix(back, front, 0.16)),
            selected: Paint::solid(selected),
            text: Paint::solid(front),
            muted: Paint::solid(mix(back, front, 0.68)),
            group: Paint::solid(mix(back, front, 0.84)),
            amber: hues.needs_you,
            glyph_amber: hues.needs_you,
            track: Paint::solid(mix(back, front, 0.25)),
            accent: hues.accent,
            teal: on_both(hues.teal),
            violet: on_both(hues.violet),
        }
    }

    /// Interface colours designed for a light terminal background (fix-light-theme-contrast
    /// D1): the sidebar a step darker than the terminal and the pane's header, the selected row in the
    /// terminal's own background, and text, needs-you and accent colours readable against
    /// both. A palette's yellow and blue are often too light to read on it as they are.
    pub fn for_light(back: Rgb, front: Rgb, hues: Hues) -> Chrome {
        let sidebar = mix(back, front, 0.06);
        let on_both = |color: Rgb, minimum: f64| {
            Paint::solid(readable(readable(color, sidebar, minimum), back, minimum))
        };
        Chrome {
            sidebar: Paint::solid(sidebar),
            sidebar_line: Paint::solid(mix(back, front, 0.2)),
            selected: Paint::solid(back),
            text: on_both(front, 4.5),
            muted: on_both(mix(back, front, 0.68), 4.5),
            group: on_both(mix(back, front, 0.84), 4.5),
            amber: on_both(hues.needs_you.rgb, 4.5),
            glyph_amber: on_both(hues.needs_you.rgb, 3.0),
            track: Paint::solid(mix(back, front, 0.3)),
            accent: on_both(hues.accent.rgb, 4.5),
            teal: on_both(hues.teal.rgb, 4.5),
            violet: on_both(hues.violet.rgb, 4.5),
        }
    }
}

/// `amount` of the way from `from` to `to`.
pub fn mix(from: Rgb, to: Rgb, amount: f64) -> Rgb {
    let channel = |a: u8, b: u8| {
        let value = f64::from(a) + (f64::from(b) - f64::from(a)) * amount;
        value.round().clamp(0.0, 255.0) as u8
    };
    Rgb {
        r: channel(from.r, to.r),
        g: channel(from.g, to.g),
        b: channel(from.b, to.b),
    }
}

/// WCAG 2.x relative luminance.
pub fn luminance(color: Rgb) -> f64 {
    let channel = |value: u8| {
        let value = f64::from(value) / 255.0;
        if value <= 0.039_28 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r) + 0.7152 * channel(color.g) + 0.0722 * channel(color.b)
}

/// WCAG 2.x contrast ratio between two colours.
pub fn contrast(a: impl Into<Paint>, b: impl Into<Paint>) -> f64 {
    let (a, b) = (a.into().rgb, b.into().rgb);
    let (high, low) = {
        let (x, y) = (luminance(a), luminance(b));
        if x > y { (x, y) } else { (y, x) }
    };
    (high + 0.05) / (low + 0.05)
}

const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
const WHITE: Rgb = Rgb {
    r: 255,
    g: 255,
    b: 255,
};

/// `text` moved toward black or white, whichever needs the smaller step, until it has
/// `minimum` contrast against `background`; unchanged when it already has. Mixing toward
/// black or white keeps its hue (fix-light-theme-contrast D3).
pub fn readable(text: Rgb, background: Rgb, minimum: f64) -> Rgb {
    if minimum <= 1.0 || contrast(text, background) >= minimum {
        return text;
    }
    // In 2 % steps: fine enough to land near the minimum, cheap enough per cell.
    let first_step = |toward: Rgb| {
        (1..=50).find_map(|step| {
            let moved = mix(text, toward, f64::from(step) / 50.0);
            (contrast(moved, background) >= minimum).then_some((step, moved))
        })
    };
    match (first_step(BLACK), first_step(WHITE)) {
        (Some((dark, darker)), Some((light, lighter))) => {
            if dark <= light {
                darker
            } else {
                lighter
            }
        }
        (Some((_, darker)), None) => darker,
        (None, Some((_, lighter))) => lighter,
        // Neither reaches it (a mid-grey background at a high minimum): the better end.
        (None, None) if contrast(BLACK, background) >= contrast(WHITE, background) => BLACK,
        (None, None) => WHITE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readable_text_meets_the_minimum_with_the_smaller_step() {
        let white = rgb(0xFFFFFF);
        // Claude Code's dark-theme amber on a light terminal darkens.
        let amber = readable(rgb(0xFFC107), white, 4.5);
        assert!(contrast(amber, white) >= 4.5);
        assert!(luminance(amber) < luminance(rgb(0xFFC107)));
        assert!(amber.r > amber.b, "still an amber, not a grey: {amber:?}");
        // Its light-theme subtle grey too.
        assert!(contrast(readable(rgb(0xAFAFAF), white, 4.5), white) >= 4.5);
        // A dark text on a dark terminal lightens.
        let dark = rgb(0x0F1115);
        let lifted = readable(rgb(0x2A2F3A), dark, 4.5);
        assert!(contrast(lifted, dark) >= 4.5);
        assert!(luminance(lifted) > luminance(rgb(0x2A2F3A)));
        // Already readable, or turned off: unchanged.
        assert_eq!(readable(rgb(0x1D2433), white, 4.5), rgb(0x1D2433));
        assert_eq!(readable(rgb(0xFFC107), white, 1.0), rgb(0xFFC107));
    }

    #[test]
    fn group_colours_stand_out_on_the_sidebar() {
        for theme in [Theme::light(), Theme::dark()] {
            for color in GroupColor::ALL {
                let ratio = contrast(theme.group_color(color), theme.chrome.sidebar);
                assert!(
                    ratio >= 3.0,
                    "{color:?} in {:?} is {ratio:.2}:1 against the sidebar",
                    theme.mode
                );
            }
        }
    }

    const AA: f64 = 4.5;

    #[test]
    fn chrome_text_meets_wcag_aa_in_both_themes() {
        for theme in [Theme::light(), Theme::dark()] {
            let chrome = theme.chrome;
            for (name, color, background) in [
                ("text", chrome.text, chrome.sidebar),
                ("muted", chrome.muted, chrome.sidebar),
                ("muted on selected", chrome.muted, chrome.selected),
                ("group", chrome.group, chrome.sidebar),
                ("amber", chrome.amber, chrome.sidebar),
                ("amber on selected", chrome.amber, chrome.selected),
                ("teal", chrome.teal, chrome.sidebar),
                ("teal on selected", chrome.teal, chrome.selected),
                ("violet", chrome.violet, chrome.sidebar),
                ("violet on selected", chrome.violet, chrome.selected),
                (
                    "terminal text",
                    theme.palette.foreground.into(),
                    theme.palette.background.into(),
                ),
            ] {
                let ratio = contrast(color, background);
                assert!(ratio >= AA, "{:?} {name}: {ratio:.2}", theme.mode);
            }
        }
    }

    /// The folder browser's tag, counts and arrows (add-folder-sessions D5): muted on a
    /// plain or hovered row, the group colour on the highlighted row's accent tint,
    /// hovered or not.
    #[test]
    fn folder_row_labels_meet_wcag_aa() {
        for theme in [Theme::light(), Theme::dark()] {
            let chrome = theme.chrome;
            let tinted = |strength| mix(chrome.selected.rgb, chrome.accent.rgb, strength);
            for (name, color, background) in [
                ("muted on a hovered row", chrome.muted, tinted(0.06)),
                ("group on the highlighted row", chrome.group, tinted(0.16)),
                (
                    "group on the hovered highlighted row",
                    chrome.group,
                    tinted(0.2),
                ),
            ] {
                let ratio = contrast(color, background);
                assert!(ratio >= AA, "{:?} {name}: {ratio:.2}", theme.mode);
            }
        }
    }

    #[test]
    fn ansi_text_colours_meet_wcag_aa() {
        for theme in [Theme::light(), Theme::dark()] {
            // Colours agents use as backgrounds rather than text in this theme.
            let backgrounds: &[usize] = match theme.mode {
                Mode::Light => &[7, 15],
                Mode::Dark => &[0],
            };
            for (index, color) in theme.palette.ansi.iter().enumerate() {
                if backgrounds.contains(&index) {
                    continue;
                }
                let ratio = contrast(*color, theme.palette.background);
                assert!(ratio >= AA, "{:?} ansi {index}: {ratio:.2}", theme.mode);
            }
        }
    }

    fn catppuccin_mocha() -> ColorSettings {
        let mut colors = ColorSettings {
            foreground: Some(0xCDD6F4),
            background: Some(0x1E1E2E),
            cursor: Some(0xF5E0DC),
            selection: Some(0xF5E0DC),
            ..ColorSettings::default()
        };
        colors.normal[3] = Some(0xF9E2AF);
        colors.dim[1] = Some(0xF38BA8);
        colors
    }

    #[test]
    fn owner_colours_replace_the_built_in_theme() {
        let theme = Theme::resolve(Mode::Light, &catppuccin_mocha());
        assert_eq!(
            theme.mode,
            Mode::Dark,
            "a dark background makes a dark theme"
        );
        assert_eq!(theme.palette.background, rgb(0x1E1E2E));
        assert_eq!(theme.palette.dim[1], Some(rgb(0xF38BA8)));
        assert_eq!(theme.palette.dim[0], None);
        assert_eq!(theme.chrome.sidebar.rgb, rgb(0x1E1E2E));
        assert_eq!(theme.chrome.amber.rgb, rgb(0xF9E2AF));
        let chrome = theme.chrome;
        for (name, color, background) in [
            ("text", chrome.text, chrome.sidebar),
            ("muted", chrome.muted, chrome.sidebar),
            ("muted on selected", chrome.muted, chrome.selected),
            ("group", chrome.group, chrome.sidebar),
            ("amber", chrome.amber, chrome.sidebar),
            ("amber on selected", chrome.amber, chrome.selected),
            ("teal", chrome.teal, chrome.sidebar),
            ("teal on selected", chrome.teal, chrome.selected),
            ("violet", chrome.violet, chrome.sidebar),
            ("violet on selected", chrome.violet, chrome.selected),
        ] {
            let ratio = contrast(color, background);
            assert!(ratio >= AA, "{name}: {ratio:.2}");
        }
    }

    #[test]
    fn partial_colours_keep_the_system_theme() {
        let mut colors = ColorSettings::default();
        colors.normal[1] = Some(0xFF0000);
        let theme = Theme::resolve(Mode::Light, &colors);
        assert_eq!(theme.mode, Mode::Light);
        assert_eq!(theme.chrome, Theme::light().chrome);
        assert_eq!(theme.palette.ansi[1], rgb(0xFF0000));
        assert_eq!(
            Theme::resolve(Mode::Dark, &ColorSettings::default()),
            Theme::dark()
        );
    }

    /// Status glyphs are graphics: WCAG 1.4.11 asks 3:1 against what is next to them.
    #[test]
    fn glyphs_meet_non_text_contrast_in_every_theme() {
        let owner = Theme::resolve(Mode::Light, &catppuccin_mocha());
        for theme in [Theme::light(), Theme::dark(), owner] {
            let chrome = theme.chrome;
            for background in [chrome.sidebar, chrome.selected] {
                for (name, color) in [
                    ("needs you", chrome.glyph_amber),
                    ("working and idle not read", chrome.text),
                    ("idle", chrome.muted),
                    ("not read yet", chrome.accent),
                    ("review", chrome.violet),
                ] {
                    let ratio = contrast(color, background);
                    assert!(ratio >= 3.0, "{:?} {name}: {ratio:.2}", theme.mode);
                }
            }
        }
    }

    /// A usage ring's filled arc must stand out from its track and the sidebar
    /// (WCAG 1.4.11, 3:1); the percentage is also written in the details.
    #[test]
    fn usage_rings_meet_non_text_contrast() {
        let owner = Theme::resolve(Mode::Light, &catppuccin_mocha());
        for theme in [Theme::light(), Theme::dark(), owner] {
            let chrome = theme.chrome;
            for (name, color) in [
                ("fill", chrome.text),
                ("near limit fill", theme.near_limit()),
                ("old near limit fill", theme.old_near_limit()),
                ("old fill", chrome.muted),
            ] {
                for (against, background) in [("sidebar", chrome.sidebar), ("track", chrome.track)]
                {
                    let ratio = contrast(color, background);
                    assert!(
                        ratio >= 3.0,
                        "{:?} {name} on {against}: {ratio:.2}",
                        theme.mode
                    );
                }
            }
        }
    }
}
