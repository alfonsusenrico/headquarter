//! The themes the owner can choose and make: the built-in ones, the templates shipped
//! with the app, the settings file's Alacritty colours and the owner's theme files, and
//! the 32 colours a theme sets (`openspec/changes/add-theme-creator/design.md`, D1, D4,
//! D6).

use std::collections::BTreeSet;
use std::path::Path;

use alacritty_terminal::vte::ansi::Rgb;
use hq_config::ColorSettings;
use hq_config::themes::{self, Listed, ThemeFile};

use crate::theme::{Chrome, Hues, Mode, Paint, Theme, luminance, mix};

/// A theme the owner can choose; kept in `app-state.json` by its `key`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeChoice {
    /// The built-in light or dark theme, following the system.
    System,
    Light,
    Dark,
    /// The settings file's colours.
    Alacritty,
    /// A template shipped with the app, by its id.
    Template(String),
    /// One of the owner's theme files, by its file name.
    File(String),
}

impl ThemeChoice {
    pub fn key(&self) -> String {
        match self {
            ThemeChoice::System => "system".to_owned(),
            ThemeChoice::Light => "light".to_owned(),
            ThemeChoice::Dark => "dark".to_owned(),
            ThemeChoice::Alacritty => "alacritty".to_owned(),
            ThemeChoice::Template(id) => format!("template:{id}"),
            ThemeChoice::File(file) => format!("file:{file}"),
        }
    }

    pub fn from_key(key: &str) -> Option<ThemeChoice> {
        Some(match key {
            "system" => ThemeChoice::System,
            "light" => ThemeChoice::Light,
            "dark" => ThemeChoice::Dark,
            "alacritty" => ThemeChoice::Alacritty,
            _ => {
                if let Some(id) = key.strip_prefix("template:") {
                    ThemeChoice::Template(id.to_owned())
                } else {
                    ThemeChoice::File(key.strip_prefix("file:")?.to_owned())
                }
            }
        })
    }
}

/// The templates shipped with the app, by id; their colours and credits are in
/// `crates/hq-app/themes/` with `NOTICE.md`.
const TEMPLATES: [(&str, &str); 8] = [
    (
        "catppuccin-mocha",
        include_str!("../themes/catppuccin-mocha.toml"),
    ),
    ("dracula", include_str!("../themes/dracula.toml")),
    ("nord", include_str!("../themes/nord.toml")),
    ("gruvbox-dark", include_str!("../themes/gruvbox-dark.toml")),
    ("tokyo-night", include_str!("../themes/tokyo-night.toml")),
    (
        "catppuccin-latte",
        include_str!("../themes/catppuccin-latte.toml"),
    ),
    (
        "solarized-light",
        include_str!("../themes/solarized-light.toml"),
    ),
    (
        "gruvbox-light",
        include_str!("../themes/gruvbox-light.toml"),
    ),
];

/// Every theme beyond the built-in ones: the templates and the owner's files.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Library {
    pub templates: Vec<(String, ThemeFile)>,
    pub files: Vec<Listed>,
}

impl Library {
    /// The templates, and the theme files in `dir` when there is one.
    pub fn load(dir: Option<&Path>) -> Library {
        let templates = TEMPLATES
            .iter()
            .filter_map(|(id, text)| Some(((*id).to_owned(), themes::parse(text).ok()?)))
            .collect();
        Library {
            templates,
            files: dir.map(themes::list).unwrap_or_default(),
        }
    }

    pub fn template(&self, id: &str) -> Option<&ThemeFile> {
        self.templates
            .iter()
            .find(|(template, _)| template == id)
            .map(|(_, file)| file)
    }

    pub fn file(&self, name: &str) -> Option<&ThemeFile> {
        self.files
            .iter()
            .find(|listed| listed.file == name)
            .and_then(|listed| listed.theme.as_ref().ok())
    }
}

/// A theme's colours and the window settings it brings, if any (D7).
#[derive(Debug, Clone, PartialEq)]
pub struct Look {
    pub theme: Theme,
    pub window_opacity: Option<f32>,
    pub window_blur: Option<bool>,
}

impl Look {
    fn plain(theme: Theme) -> Look {
        Look {
            theme,
            window_opacity: None,
            window_blur: None,
        }
    }
}

/// The choice in effect. With none kept: Alacritty when the settings file has colours,
/// System otherwise. A kept choice that no longer exists falls back to System.
pub fn effective(
    choice: Option<&ThemeChoice>,
    colors: &ColorSettings,
    library: &Library,
) -> ThemeChoice {
    let has_colours = !colors.is_empty();
    match choice {
        None if has_colours => ThemeChoice::Alacritty,
        None => ThemeChoice::System,
        Some(ThemeChoice::Alacritty) if !has_colours => ThemeChoice::System,
        Some(ThemeChoice::Template(id)) if library.template(id).is_none() => ThemeChoice::System,
        Some(ThemeChoice::File(file)) if library.file(file).is_none() => ThemeChoice::System,
        Some(choice) => choice.clone(),
    }
}

/// The look for the owner's `choice` with the system in `system` mode.
pub fn look(
    choice: Option<&ThemeChoice>,
    system: Mode,
    colors: &ColorSettings,
    library: &Library,
) -> Look {
    match effective(choice, colors, library) {
        ThemeChoice::System => Look::plain(Theme::for_mode(system)),
        ThemeChoice::Light => Look::plain(Theme::light()),
        ThemeChoice::Dark => Look::plain(Theme::dark()),
        ThemeChoice::Alacritty => Look::plain(Theme::resolve(system, colors)),
        ThemeChoice::Template(id) => library.template(&id).map_or_else(
            || Look::plain(Theme::for_mode(system)),
            |file| from_file(file, system),
        ),
        ThemeChoice::File(name) => library.file(&name).map_or_else(
            || Look::plain(Theme::for_mode(system)),
            |file| from_file(file, system),
        ),
    }
}

/// A theme file's look: its terminal colours, the interface made from them, then any
/// interface colours and opacities the file sets itself.
pub fn from_file(file: &ThemeFile, system: Mode) -> Look {
    let mut theme = Theme::resolve(system, &file.colors);
    for token in Token::all() {
        let Some(key) = token.interface_key() else {
            continue;
        };
        if let Some(value) = file.interface.get(key) {
            let current = token.get(&theme);
            token.set(
                &mut theme,
                Paint {
                    rgb: crate::theme::rgb(*value),
                    ..current
                },
            );
        }
    }
    for token in Token::all() {
        if let Some(opacity) = file.opacity.get(&token.file_key()) {
            let current = token.get(&theme);
            token.set(&mut theme, current.with_opacity(*opacity));
        }
    }
    Look {
        theme,
        window_opacity: file.window_opacity,
        window_blur: file.window_blur,
    }
}

/// The file for a theme the owner made: every colour written out, opacity where it is
/// below 100 %, and the window settings.
pub fn to_file(look: &Look, name: &str) -> ThemeFile {
    let hex = |rgb: Rgb| (u32::from(rgb.r) << 16) | (u32::from(rgb.g) << 8) | u32::from(rgb.b);
    let palette = &look.theme.palette;
    let mut colors = ColorSettings {
        foreground: Some(hex(palette.foreground)),
        background: Some(hex(palette.background)),
        cursor: Some(hex(palette.cursor)),
        selection: Some(hex(palette.selection)),
        selection_text: palette.selection_text.map(hex),
        ..ColorSettings::default()
    };
    for index in 0..8 {
        colors.normal[index] = Some(hex(palette.ansi[index]));
        colors.bright[index] = Some(hex(palette.ansi[index + 8]));
    }
    let mut file = ThemeFile {
        name: name.trim().to_owned(),
        credit: None,
        colors,
        window_opacity: look.window_opacity,
        window_blur: look.window_blur,
        ..ThemeFile::default()
    };
    for token in Token::all() {
        let paint = token.get(&look.theme);
        if let Some(key) = token.interface_key() {
            file.interface.insert(key.to_owned(), hex(paint.rgb));
        }
        if paint.opacity < 100 {
            file.opacity.insert(token.file_key(), paint.opacity);
        }
    }
    file
}

/// One colour a theme sets, as the creator lists it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Token {
    Sidebar,
    Raised,
    Lines,
    Tracks,
    Text,
    Muted,
    Groups,
    Accent,
    NeedsYou,
    NeedsYouIcon,
    BackgroundWork,
    Review,
    Background,
    Foreground,
    Cursor,
    Selection,
    /// A terminal colour, 0–7 normal and 8–15 bright.
    Terminal(u8),
}

/// The four colours the others are made from (D4).
pub const SEEDS: [Token; 4] = [Token::Sidebar, Token::Text, Token::Accent, Token::NeedsYou];

impl Token {
    pub fn all() -> Vec<Token> {
        let mut all = vec![
            Token::Sidebar,
            Token::Raised,
            Token::Lines,
            Token::Tracks,
            Token::Text,
            Token::Muted,
            Token::Groups,
            Token::Accent,
            Token::NeedsYou,
            Token::NeedsYouIcon,
            Token::BackgroundWork,
            Token::Review,
            Token::Background,
            Token::Foreground,
            Token::Cursor,
            Token::Selection,
        ];
        all.extend((0..16).map(Token::Terminal));
        all
    }

    pub fn label(self) -> String {
        match self {
            Token::Sidebar => "Sidebar and panels".to_owned(),
            Token::Raised => "Raised surfaces".to_owned(),
            Token::Lines => "Dividers and borders".to_owned(),
            Token::Tracks => "Tracks and outlines".to_owned(),
            Token::Text => "Text".to_owned(),
            Token::Muted => "Secondary text".to_owned(),
            Token::Groups => "Group headers".to_owned(),
            Token::Accent => "Accent".to_owned(),
            Token::NeedsYou => "Needs you: words".to_owned(),
            Token::NeedsYouIcon => "Needs you: icon".to_owned(),
            Token::BackgroundWork => "Background work".to_owned(),
            Token::Review => "Plan to review".to_owned(),
            Token::Background => "Background".to_owned(),
            Token::Foreground => "Text".to_owned(),
            Token::Cursor => "Cursor".to_owned(),
            Token::Selection => "Selection".to_owned(),
            Token::Terminal(index) => {
                let name = themes::TERMINAL_NAMES[usize::from(index % 8)];
                let mut name = name.to_owned();
                name[..1].make_ascii_uppercase();
                if index >= 8 {
                    format!("Bright {}", name.to_lowercase())
                } else {
                    name
                }
            }
        }
    }

    /// Where the colour shows, under its name in the creator.
    pub fn use_text(self) -> &'static str {
        match self {
            Token::Sidebar => "The sidebar, the session header and side columns",
            Token::Raised => "The open row and windows such as this one",
            Token::Lines => "Lines between areas, field and window borders",
            Token::Tracks => "Spinner and usage-ring tracks, tag and key outlines",
            Token::Text => "Titles and names",
            Token::Muted => "Ages, folders, hints and idle icons",
            Token::Groups => "Group names in the sidebar",
            Token::Accent => "The unread dot, picks and highlights",
            Token::NeedsYou => "\"needs you\" and near-limit usage",
            Token::NeedsYouIcon => "The ring on a session that needs you",
            Token::BackgroundWork => "\"background\" on a session whose work still runs",
            Token::Review => "\"review\" and its diamond on a plan to review",
            Token::Background => "Behind the agent's screen",
            Token::Foreground => "The agent's default text",
            Token::Cursor => "The text cursor",
            Token::Selection => "Selected text",
            Token::Terminal(_) => "What programs get when they ask for this colour by name",
        }
    }

    pub fn group(self) -> &'static str {
        match self {
            Token::NeedsYou | Token::NeedsYouIcon | Token::BackgroundWork | Token::Review => {
                "Status"
            }
            Token::Background | Token::Foreground | Token::Cursor | Token::Selection => "Terminal",
            Token::Terminal(_) => "Terminal colours",
            _ => "Interface",
        }
    }

    /// The colour's name in a theme file's `[headquarter.colors]`.
    pub fn interface_key(self) -> Option<&'static str> {
        let index = match self {
            Token::Sidebar => 0,
            Token::Raised => 1,
            Token::Lines => 2,
            Token::Tracks => 3,
            Token::Text => 4,
            Token::Muted => 5,
            Token::Groups => 6,
            Token::Accent => 7,
            Token::NeedsYou => 8,
            Token::NeedsYouIcon => 9,
            Token::BackgroundWork => 10,
            Token::Review => 11,
            _ => return None,
        };
        Some(themes::INTERFACE_COLOURS[index])
    }

    /// The colour's name in a theme file's `[headquarter.opacity]`.
    pub fn file_key(self) -> String {
        match self {
            Token::Background => "background".to_owned(),
            Token::Foreground => "foreground".to_owned(),
            Token::Cursor => "cursor".to_owned(),
            Token::Selection => "selection".to_owned(),
            Token::Terminal(index) => {
                let set = if index >= 8 { "bright" } else { "normal" };
                format!("{set}.{}", themes::TERMINAL_NAMES[usize::from(index % 8)])
            }
            interface => interface.interface_key().unwrap_or_default().to_owned(),
        }
    }

    pub fn get(self, theme: &Theme) -> Paint {
        let chrome = &theme.chrome;
        let palette = &theme.palette;
        let terminal = |rgb: Rgb, opacity: u8| Paint { rgb, opacity };
        match self {
            Token::Sidebar => chrome.sidebar,
            Token::Raised => chrome.selected,
            Token::Lines => chrome.sidebar_line,
            Token::Tracks => chrome.track,
            Token::Text => chrome.text,
            Token::Muted => chrome.muted,
            Token::Groups => chrome.group,
            Token::Accent => chrome.accent,
            Token::NeedsYou => chrome.amber,
            Token::NeedsYouIcon => chrome.glyph_amber,
            Token::BackgroundWork => chrome.teal,
            Token::Review => chrome.violet,
            Token::Background => terminal(palette.background, palette.opacity.background),
            Token::Foreground => terminal(palette.foreground, palette.opacity.foreground),
            Token::Cursor => terminal(palette.cursor, palette.opacity.cursor),
            Token::Selection => terminal(palette.selection, palette.opacity.selection),
            Token::Terminal(index) => {
                let index = usize::from(index);
                terminal(palette.ansi[index], palette.opacity.ansi[index])
            }
        }
    }

    pub fn set(self, theme: &mut Theme, paint: Paint) {
        let chrome = &mut theme.chrome;
        let palette = &mut theme.palette;
        match self {
            Token::Sidebar => chrome.sidebar = paint,
            Token::Raised => chrome.selected = paint,
            Token::Lines => chrome.sidebar_line = paint,
            Token::Tracks => chrome.track = paint,
            Token::Text => chrome.text = paint,
            Token::Muted => chrome.muted = paint,
            Token::Groups => chrome.group = paint,
            Token::Accent => chrome.accent = paint,
            Token::NeedsYou => chrome.amber = paint,
            Token::NeedsYouIcon => chrome.glyph_amber = paint,
            Token::BackgroundWork => chrome.teal = paint,
            Token::Review => chrome.violet = paint,
            Token::Background => {
                palette.background = paint.rgb;
                palette.opacity.background = paint.opacity;
            }
            Token::Foreground => {
                palette.foreground = paint.rgb;
                palette.opacity.foreground = paint.opacity;
            }
            Token::Cursor => {
                palette.cursor = paint.rgb;
                palette.opacity.cursor = paint.opacity;
            }
            Token::Selection => {
                palette.selection = paint.rgb;
                palette.opacity.selection = paint.opacity;
            }
            Token::Terminal(index) => {
                let index = usize::from(index);
                palette.ansi[index] = paint.rgb;
                palette.opacity.ansi[index] = paint.opacity;
            }
        }
    }
}

/// Makes every colour that is not pinned from the four seeds, keeping each colour's own
/// opacity (D4). The terminal colours are never made from seeds.
pub fn apply_seeds(theme: &mut Theme, pinned: &BTreeSet<Token>) {
    let back = Token::Sidebar.get(theme).rgb;
    let front = Token::Text.get(theme).rgb;
    let accent = Token::Accent.get(theme);
    let needs_you = Token::NeedsYou.get(theme);
    let hues = Hues {
        accent,
        needs_you,
        teal: Token::BackgroundWork.get(theme),
        violet: Token::Review.get(theme),
    };
    let made = Chrome::from_colours(back, front, hues);
    let derived = [
        (Token::Raised, made.selected.rgb),
        (Token::Lines, made.sidebar_line.rgb),
        (Token::Tracks, made.track.rgb),
        (Token::Muted, made.muted.rgb),
        (Token::Groups, made.group.rgb),
        (Token::NeedsYouIcon, needs_you.rgb),
        (Token::BackgroundWork, made.teal.rgb),
        (Token::Review, made.violet.rgb),
        (Token::Background, back),
        (Token::Foreground, front),
        (Token::Cursor, front),
        (Token::Selection, mix(back, accent.rgb, 0.3)),
    ];
    for (token, rgb) in derived {
        if !pinned.contains(&token) {
            let opacity = token.get(theme).opacity;
            token.set(theme, Paint { rgb, opacity });
        }
    }
    theme.mode = if luminance(back) < 0.18 {
        Mode::Dark
    } else {
        Mode::Light
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::contrast;

    /// Every light theme shipped, built-in or a template, keeps its interface readable and
    /// apart from the terminal; dark templates keep the chrome they had
    /// (fix-light-theme-contrast D1).
    #[test]
    fn light_themes_are_designed_for_light() {
        let library = Library::load(None);
        let mut light = vec![("light".to_owned(), Theme::light())];
        for (id, file) in &library.templates {
            let theme = from_file(file, Mode::Light).theme;
            match theme.mode {
                Mode::Light => light.push((id.clone(), theme)),
                Mode::Dark => {
                    let chrome = Chrome::from_colours(
                        theme.palette.background,
                        theme.palette.foreground,
                        Hues::from_palette(&Theme::dark().chrome, &file.colors),
                    );
                    assert_eq!(theme.chrome, chrome, "{id} is unchanged");
                }
            }
        }
        assert_eq!(
            light.len(),
            4,
            "Light, Latte, Solarized Light, Gruvbox Light"
        );
        for (id, theme) in light {
            let c = theme.chrome;
            let terminal = crate::theme::Paint::solid(theme.palette.background);
            for (name, color, minimum) in [
                ("text", c.text, 4.5),
                ("muted", c.muted, 4.5),
                ("group", c.group, 4.5),
                ("amber", c.amber, 4.5),
                ("accent", c.accent, 4.5),
                ("teal", c.teal, 4.5),
                ("violet", c.violet, 4.5),
                ("glyph amber", c.glyph_amber, 3.0),
            ] {
                for (surface_name, surface) in [
                    ("sidebar", c.sidebar),
                    ("selected", c.selected),
                    ("terminal", terminal),
                ] {
                    let ratio = contrast(color, surface);
                    assert!(
                        ratio >= minimum,
                        "{id}: {name} on {surface_name} is {ratio:.2}:1"
                    );
                }
            }
            assert!(
                crate::theme::luminance(c.sidebar.rgb) < crate::theme::luminance(terminal.rgb),
                "{id}: the sidebar sits apart from the terminal"
            );
        }
    }
    use crate::theme::rgb;

    fn library() -> Library {
        Library::load(None)
    }

    #[test]
    fn all_eight_templates_load_with_sixteen_colours_and_a_credit() {
        let library = library();
        assert_eq!(library.templates.len(), 8);
        for (id, file) in &library.templates {
            assert!(
                file.colors
                    .normal
                    .iter()
                    .chain(&file.colors.bright)
                    .all(Option::is_some),
                "{id}"
            );
            assert!(
                file.colors.background.is_some() && file.colors.foreground.is_some(),
                "{id}"
            );
            assert!(
                file.credit
                    .as_deref()
                    .is_some_and(|credit| credit.contains(" · ")),
                "{id}"
            );
        }
        let nord = from_file(library.template("nord").unwrap(), Mode::Light);
        assert_eq!(
            nord.theme.palette.background,
            rgb(0x2E3440),
            "Nord's own background"
        );
        assert_eq!(nord.theme.chrome.accent.rgb, rgb(0x81A1C1), "Nord's blue");
        assert_eq!(nord.theme.mode, Mode::Dark);
        let latte = from_file(library.template("catppuccin-latte").unwrap(), Mode::Dark);
        assert_eq!(latte.theme.mode, Mode::Light);
    }

    #[test]
    fn choices_round_trip_through_their_keys() {
        for choice in [
            ThemeChoice::System,
            ThemeChoice::Light,
            ThemeChoice::Dark,
            ThemeChoice::Alacritty,
            ThemeChoice::Template("nord".to_owned()),
            ThemeChoice::File("mine.toml".to_owned()),
        ] {
            assert_eq!(ThemeChoice::from_key(&choice.key()), Some(choice));
        }
        assert_eq!(ThemeChoice::from_key("solarized"), None);
    }

    #[test]
    fn missing_choices_fall_back_to_system() {
        let library = library();
        let none = ColorSettings::default();
        let gone = ThemeChoice::File("gone.toml".to_owned());
        assert_eq!(effective(Some(&gone), &none, &library), ThemeChoice::System);
        let unknown = ThemeChoice::Template("solarized-dark".to_owned());
        assert_eq!(
            effective(Some(&unknown), &none, &library),
            ThemeChoice::System
        );
        let dracula = ThemeChoice::Template("dracula".to_owned());
        assert_eq!(effective(Some(&dracula), &none, &library), dracula);
        assert_eq!(
            effective(Some(&ThemeChoice::Alacritty), &none, &library),
            ThemeChoice::System
        );
    }

    #[test]
    fn a_made_theme_round_trips_through_its_file() {
        let mut theme = Theme::dark();
        Token::Accent.set(
            &mut theme,
            Paint {
                rgb: rgb(0xFAB387),
                opacity: 100,
            },
        );
        Token::Sidebar.set(
            &mut theme,
            Paint {
                rgb: rgb(0x1E1E2E),
                opacity: 80,
            },
        );
        Token::Terminal(1).set(
            &mut theme,
            Paint {
                rgb: rgb(0xF38BA8),
                opacity: 60,
            },
        );
        let look = Look {
            theme,
            window_opacity: Some(0.9),
            window_blur: Some(true),
        };
        let file = to_file(&look, " Mocha orange ");
        assert_eq!(file.name, "Mocha orange");
        assert_eq!(file.opacity.get("sidebar"), Some(&80));
        assert_eq!(file.opacity.get("normal.red"), Some(&60));
        let back = from_file(
            &themes::parse_pair(
                &themes::render_colours(&file, "x.headquarter.toml"),
                Some(&themes::render_companion(&file)),
                "x.toml",
            )
            .unwrap(),
            Mode::Light,
        );
        for token in Token::all() {
            assert_eq!(token.get(&back.theme), token.get(&look.theme), "{token:?}");
        }
        assert_eq!(
            (back.window_opacity, back.window_blur),
            (Some(0.9), Some(true))
        );
    }

    #[test]
    fn seeds_make_the_colours_that_are_not_pinned() {
        let mut theme = Theme::dark();
        let lines_before = Token::Lines.get(&theme);
        let pinned = BTreeSet::from([Token::Lines]);
        Token::Sidebar.set(
            &mut theme,
            Paint {
                rgb: rgb(0xFDF6E3),
                opacity: 90,
            },
        );
        Token::Text.set(
            &mut theme,
            Paint {
                rgb: rgb(0x586E75),
                opacity: 100,
            },
        );
        Token::Muted.set(
            &mut theme,
            Paint {
                rgb: rgb(0x000000),
                opacity: 70,
            },
        );
        apply_seeds(&mut theme, &pinned);
        assert_eq!(
            Token::Lines.get(&theme),
            lines_before,
            "a pinned colour stays"
        );
        assert_eq!(Token::Background.get(&theme).rgb, rgb(0xFDF6E3));
        assert_eq!(Token::Foreground.get(&theme).rgb, rgb(0x586E75));
        assert_eq!(
            Token::Muted.get(&theme),
            Paint {
                rgb: mix(rgb(0xFDF6E3), rgb(0x586E75), 0.68),
                opacity: 70
            },
            "made from the seeds, keeping its own opacity"
        );
        assert_eq!(
            theme.mode,
            Mode::Light,
            "a light background makes a light theme"
        );
        assert_eq!(
            Token::Terminal(1).get(&theme),
            Token::Terminal(1).get(&Theme::dark())
        );
    }

    #[test]
    fn tokens_have_unique_file_keys_and_labels() {
        let keys: BTreeSet<String> = Token::all().into_iter().map(Token::file_key).collect();
        assert_eq!(keys.len(), 32);
        assert_eq!(Token::Terminal(9).label(), "Bright red");
        assert_eq!(Token::Terminal(0).label(), "Black");
    }

    /// "background" and "review" read at 4.5:1 on the sidebar and the open row in every
    /// theme shipped, made from each palette's cyan and magenta (add-background-work-status D8).
    #[test]
    fn background_and_review_colours_read_in_every_theme() {
        let library = library();
        let mut themes = vec![
            ("light".to_owned(), Theme::light()),
            ("dark".to_owned(), Theme::dark()),
        ];
        for (id, file) in &library.templates {
            themes.push((id.clone(), from_file(file, Mode::Dark).theme));
        }
        for (id, theme) in themes {
            let c = theme.chrome;
            for (name, color) in [("teal", c.teal), ("violet", c.violet)] {
                for (surface_name, surface) in [("sidebar", c.sidebar), ("selected", c.selected)] {
                    let ratio = crate::theme::contrast(color, surface);
                    assert!(
                        ratio >= 4.5,
                        "{id}: {name} on {surface_name} is {ratio:.2}:1"
                    );
                }
            }
            assert_ne!(c.teal, c.violet, "{id}");
        }
        let nord = from_file(library.template("nord").unwrap(), Mode::Dark);
        assert_eq!(nord.theme.chrome.teal.rgb, rgb(0x88C0D0), "Nord's cyan");
    }

    #[test]
    fn theme_files_keep_their_background_and_review_colours() {
        let mut look = from_file(library().template("dracula").unwrap(), Mode::Dark);
        Token::BackgroundWork.set(&mut look.theme, Paint::solid(rgb(0x00FFAA)));
        let file = to_file(&look, "Mine");
        assert_eq!(file.interface.get("background_work"), Some(&0x00FFAA));
        let back = from_file(&file, Mode::Dark);
        assert_eq!(back.theme.chrome.teal.rgb, rgb(0x00FFAA));
        assert_eq!(back.theme.chrome.violet, look.theme.chrome.violet);
    }
}
