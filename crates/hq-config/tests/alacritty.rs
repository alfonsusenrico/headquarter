//! The owner's Alacritty settings, copied in as a fixture, load as Headquarter settings.

use std::path::{Path, PathBuf};

use hq_config::{CursorShape, Decorations, KeyBinding, OptionAsAlt, Settings, SettingsError};

const AGENTS: &[&str] = &["claude", "codex", "agy"];

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn parse_text(source: &str) -> Result<Settings, SettingsError> {
    let home = fixtures().join("home");
    hq_config::parse(
        source,
        &home.join(".config/headquarter/config.toml"),
        AGENTS,
        Some(&home),
    )
}

#[test]
fn owner_alacritty_settings_carry_over() {
    let source = std::fs::read_to_string(fixtures().join("alacritty.toml")).unwrap();
    let settings = parse_text(&source).unwrap();
    let look = &settings.appearance;

    assert!((look.opacity - 0.7).abs() < f32::EPSILON);
    assert!(look.blur);
    assert!(look.live_reload);
    assert_eq!(look.padding, (10, 10));
    assert_eq!(look.decorations, Decorations::Buttonless);
    assert_eq!(look.option_as_alt, OptionAsAlt::Both);
    assert_eq!(look.cursor, Some(CursorShape::Beam));
    assert_eq!(look.font.family.as_deref(), Some("JetBrainsMono Nerd Font"));
    assert_eq!(look.font.size, Some(14.0));
    assert_eq!(look.font.normal_weight, Some(500));
    assert_eq!(look.font.bold_weight, Some(900));
    assert_eq!(look.font.italic_weight, Some(400));
    assert_eq!(look.font.bold_italic_weight, Some(900));
    assert_eq!(
        look.bindings,
        [KeyBinding {
            key: "enter".to_owned(),
            control: false,
            alt: false,
            shift: true,
            platform: false,
            chars: b"\x1b\r".to_vec(),
        }]
    );

    // Catppuccin Mocha, from the imported file.
    assert_eq!(look.colors.background, Some(0x1E1E2E));
    assert_eq!(look.colors.foreground, Some(0xCDD6F4));
    assert_eq!(look.colors.cursor, Some(0xF5E0DC));
    assert_eq!(look.colors.selection, Some(0xF5E0DC));
    assert_eq!(look.colors.normal[1], Some(0xF38BA8));
    assert_eq!(look.colors.bright[0], Some(0x585B70));
    assert_eq!(look.colors.dim[7], Some(0xBAC2DE));
    assert_eq!(
        settings.imports,
        [fixtures().join("home/.config/alacritty/alacritty-theme/themes/catppuccin_mocha.toml")]
    );

    // `[env]` is reported, with its line, rather than silently ignored.
    assert_eq!(settings.warnings.len(), 1, "{:?}", settings.warnings);
    assert_eq!(settings.warnings[0].line, Some(5));
    assert!(settings.warnings[0].message.contains("`env`"));
}

#[test]
fn own_colours_override_imported_ones() {
    let settings = parse_text(
        "[general]\nimport = [\"~/.config/alacritty/alacritty-theme/themes/catppuccin_mocha.toml\"]\n\n[colors.primary]\nbackground = \"0x000000\"\n",
    )
    .unwrap();
    assert_eq!(settings.appearance.colors.background, Some(0x000000));
    assert_eq!(settings.appearance.colors.foreground, Some(0xCDD6F4));
}

#[test]
fn relative_imports_resolve_against_the_settings_folder() {
    let error = parse_text("[general]\nimport = [\"themes/missing.toml\"]\n").unwrap_err();
    assert_eq!(error.line, Some(2));
    assert!(
        error
            .message
            .contains(".config/headquarter/themes/missing.toml"),
        "{error}"
    );
}

#[test]
fn defaults_without_appearance_keys() {
    let settings = parse_text("").unwrap();
    assert_eq!(settings.appearance.opacity, 1.0);
    assert!(!settings.appearance.blur);
    assert_eq!(settings.appearance.decorations, Decorations::Full);
    assert_eq!(settings.appearance.option_as_alt, OptionAsAlt::None);
    assert_eq!(settings.appearance.dim_unfocused, 0.0);
    assert_eq!(settings.appearance.minimum_contrast, 4.5);
    assert!(settings.appearance.colors.is_empty());
    assert!(settings.warnings.is_empty());
}

#[test]
fn the_minimum_contrast_is_a_window_setting() {
    let set = parse_text("[window]\nminimum_contrast = 3.0\n").unwrap();
    assert_eq!(set.appearance.minimum_contrast, 3.0);
    assert!(set.warnings.is_empty());
    let off = parse_text("[window]\nminimum_contrast = 1\n").unwrap();
    assert_eq!(off.appearance.minimum_contrast, 1.0);
}

#[test]
fn invalid_values_name_their_line() {
    let cases = [
        ("[window]\nopacity = 1.5\n", "opacity must be between"),
        (
            "[window]\ndim_unfocused = 0.9\n",
            "dim_unfocused must be between",
        ),
        (
            "[window]\nminimum_contrast = 0.5\n",
            "minimum_contrast must be between",
        ),
        ("[window]\ndecorations = \"Fancy\"\n", "decorations must be"),
        (
            "[window]\noption_as_alt = \"Maybe\"\n",
            "option_as_alt must be",
        ),
        ("[cursor]\nstyle = \"Triangle\"\n", "cursor style must be"),
        (
            "[colors.primary]\nbackground = \"blue\"\n",
            "is not a colour",
        ),
        ("[font]\nsize = 200.0\n", "font size must be between"),
        (
            "[[keyboard.bindings]]\nkey = \"Bogus\"\nchars = \"x\"\n",
            "unknown key `Bogus`",
        ),
        ("[window]\nopacity = \"high\"\n", "invalid type"),
    ];
    for (source, expected) in cases {
        let error = parse_text(source).unwrap_err();
        assert_eq!(error.line, Some(2), "{source}: {error}");
        assert!(error.message.contains(expected), "{source}: {error}");
    }
}

#[test]
fn unsupported_and_unknown_keys() {
    let settings = parse_text(
        "[scrolling]\nhistory = 10000\n\n[[keyboard.bindings]]\nkey = \"N\"\nmods = \"Command\"\naction = \"CreateNewWindow\"\n",
    )
    .unwrap();
    let lines: Vec<_> = settings
        .warnings
        .iter()
        .map(|warning| warning.line)
        .collect();
    assert_eq!(
        lines,
        [Some(1), Some(5), Some(7)],
        "{:?}",
        settings.warnings
    );
    assert!(settings.appearance.bindings.is_empty());

    let error = parse_text("[window]\nopacityy = 0.5\n").unwrap_err();
    assert_eq!(error.line, Some(2));
    assert!(error.message.contains("unknown setting `window.opacityy`"));
}

#[test]
fn bindings_read_alacritty_modifiers() {
    let settings = parse_text(
        "[[keyboard.bindings]]\nkey = \"K\"\nmods = \"Control|Shift\"\nchars = \"\\u000b\"\n",
    )
    .unwrap();
    let binding = &settings.appearance.bindings[0];
    assert_eq!(binding.key, "k");
    assert!(binding.control && binding.shift && !binding.alt && !binding.platform);
    assert_eq!(binding.chars, [0x0b]);
}

#[test]
fn cursor_style_as_a_table() {
    let settings = parse_text("[cursor.style]\nshape = \"Underline\"\n").unwrap();
    assert_eq!(settings.appearance.cursor, Some(CursorShape::Underline));
}
