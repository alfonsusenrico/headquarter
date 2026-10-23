//! The settings file's appearance and keyboard settings, turned into what the window
//! and the terminal pane use (`openspec/changes/add-terminal-pane/design.md`, D6).

use std::time::Duration;

use alacritty_terminal::vte::ansi::CursorShape;
use gpui::{App, FontWeight, SharedString, Window, WindowBackgroundAppearance};
use hq_config::{
    Appearance, Decorations, FontSettings, Notifications, OptionAsAlt, SettingsError,
    SettingsWatcher,
};

use crate::terminal::keys::Binding;
use crate::terminal::view::{PaneOptions, default_font};
use crate::theme::Mode;

/// How often the settings file is checked for edits; one `stat` per file each time.
pub const RELOAD_INTERVAL: Duration = Duration::from_secs(1);

/// The settings file, whether or not it exists yet.
pub fn path() -> Option<std::path::PathBuf> {
    hq_config::settings_path()
}

pub fn watcher() -> SettingsWatcher {
    SettingsWatcher::new(
        path(),
        hq_discovery::KNOWN_AGENTS
            .iter()
            .map(|agent| agent.id)
            .collect(),
    )
}

/// Which notifications to post; an invalid file keeps the last good choice, as `read`
/// does for the look.
pub fn notifications(
    watcher: &mut SettingsWatcher,
    last_good: Option<Notifications>,
) -> Notifications {
    match watcher.current() {
        Ok(settings) => settings.notifications,
        Err(_) => last_good.unwrap_or_default(),
    }
}

/// The settings to use, and what to tell the owner about the file. An invalid file
/// leaves the last good settings in force, as Alacritty does.
pub fn read(
    watcher: &mut SettingsWatcher,
    last_good: Option<&Appearance>,
) -> (Appearance, Option<String>) {
    match watcher.current() {
        Ok(settings) => {
            let mut notes: Vec<String> =
                settings.warnings.iter().map(ToString::to_string).collect();
            notes.extend(font_notes(&settings.appearance, &has_face));
            (settings.appearance, settings_note(&notes))
        }
        Err(error) => (
            last_good.cloned().unwrap_or_default(),
            Some(error_note(&error)),
        ),
    }
}

fn error_note(error: &SettingsError) -> String {
    format!("Settings not applied: {error}")
}

fn settings_note(notes: &[String]) -> Option<String> {
    let first = notes.first()?;
    let more = notes.len() - 1;
    Some(if more == 0 {
        format!("Settings: {first}")
    } else {
        format!("Settings: {first} (and {more} more; see `hq agents`)")
    })
}

/// Whether the font family has a face with exactly this style name; `None` when that
/// cannot be told. Alacritty looks styles up this way on macOS (crossfont); on Linux it
/// takes fontconfig's nearest match, which GPUI does too, so nothing is checked there.
type FaceLookup = dyn Fn(&str, &str) -> Option<bool>;

#[cfg(target_os = "macos")]
fn has_face(family: &str, style: &str) -> Option<bool> {
    let descriptors = core_text::font_collection::create_for_family(family)?.get_descriptors()?;
    Some(
        descriptors
            .iter()
            .any(|descriptor| descriptor.style_name() == style),
    )
}

#[cfg(not(target_os = "macos"))]
fn has_face(_family: &str, _style: &str) -> Option<bool> {
    None
}

/// The bold, italic or bold italic style as the settings file names it.
struct NamedStyle<'a> {
    key: &'static str,
    words: &'static str,
    style: Option<&'a str>,
    weight: Option<u16>,
    default_weight: FontWeight,
    slanted: bool,
}

fn named_styles(font: &FontSettings) -> [NamedStyle<'_>; 3] {
    let defaults = PaneOptions::default();
    [
        NamedStyle {
            key: "font.bold",
            words: "bold",
            style: font.bold_style.as_deref(),
            weight: font.bold_weight,
            default_weight: defaults.bold_weight,
            slanted: false,
        },
        NamedStyle {
            key: "font.italic",
            words: "italic",
            style: font.italic_style.as_deref(),
            weight: font.italic_weight,
            default_weight: defaults.italic_weight,
            slanted: true,
        },
        NamedStyle {
            key: "font.bold_italic",
            words: "bold italic",
            style: font.bold_italic_style.as_deref(),
            weight: font.bold_italic_weight,
            default_weight: defaults.bold_italic_weight,
            slanted: true,
        },
    ]
}

fn family_of(font: &FontSettings) -> String {
    font.family
        .clone()
        .unwrap_or_else(|| default_font().to_owned())
}

/// Whether a style falls back to the normal face: its name names no face of the family
/// (fix-font-styles-like-alacritty D1).
fn falls_back(family: &str, style: Option<&str>, lookup: &FaceLookup) -> bool {
    style.is_some_and(|style| lookup(family, style) == Some(false))
}

/// One note per style the font has no face for (D3).
fn font_notes(appearance: &Appearance, lookup: &FaceLookup) -> Vec<String> {
    let family = family_of(&appearance.font);
    named_styles(&appearance.font)
        .into_iter()
        .filter(|named| falls_back(&family, named.style, lookup))
        .map(|named| {
            format!(
                "{} style \"{}\" is not in {family}; {} text uses the normal face, as in Alacritty",
                named.key,
                named.style.unwrap_or_default(),
                named.words,
            )
        })
        .collect()
}

pub fn pane_options(appearance: &Appearance) -> PaneOptions {
    pane_options_with(appearance, &has_face)
}

fn pane_options_with(appearance: &Appearance, lookup: &FaceLookup) -> PaneOptions {
    let defaults = PaneOptions::default();
    let weight = |weight: Option<u16>, default: FontWeight| {
        weight.map_or(default, |weight| FontWeight(f32::from(weight)))
    };
    let font = &appearance.font;
    let family = family_of(font);
    let normal_weight = weight(font.normal_weight, defaults.normal_weight);
    // Each style as (weight, slanted).
    let [bold, italic, bold_italic] = named_styles(font).map(|named| {
        if falls_back(&family, named.style, lookup) {
            (normal_weight, false)
        } else {
            (weight(named.weight, named.default_weight), named.slanted)
        }
    });
    PaneOptions {
        font_family: SharedString::from(family),
        font_size: font.size.unwrap_or(defaults.font_size),
        normal_weight,
        bold_weight: bold.0,
        italic_weight: italic.0,
        bold_italic_weight: bold_italic.0,
        italic_slanted: italic.1,
        bold_italic_slanted: bold_italic.1,
        padding: (
            f32::from(appearance.padding.0),
            f32::from(appearance.padding.1),
        ),
        // GPUI does not say which Option key is held, so any side makes both act as Alt.
        alt_is_meta: appearance.option_as_alt != OptionAsAlt::None,
        bindings: appearance
            .bindings
            .iter()
            .map(|binding| Binding {
                key: binding.key.clone(),
                control: binding.control,
                alt: binding.alt,
                shift: binding.shift,
                platform: binding.platform,
                chars: binding.chars.clone(),
            })
            .collect(),
        dim_unfocused: appearance.dim_unfocused,
        opacity: appearance.opacity,
        cursor: appearance.cursor.map(|shape| match shape {
            hq_config::CursorShape::Block => CursorShape::Block,
            hq_config::CursorShape::Beam => CursorShape::Beam,
            hq_config::CursorShape::Underline => CursorShape::Underline,
        }),
    }
}

pub fn window_background(appearance: &Appearance) -> WindowBackgroundAppearance {
    if appearance.opacity >= 1.0 {
        WindowBackgroundAppearance::Opaque
    } else if appearance.blur {
        WindowBackgroundAppearance::Blurred
    } else {
        WindowBackgroundAppearance::Transparent
    }
}

/// Switches the UI kit's controls to light or dark. The kit's root view paints its
/// theme background under everything, which would hide a translucent or blurred
/// window, so that one token is cleared after every switch; the sidebar and the pane
/// paint their own backgrounds at the window's opacity.
pub fn set_controls_mode(mode: Mode, window: &mut Window, cx: &mut App) {
    let mode = match mode {
        Mode::Light => gpui_component::ThemeMode::Light,
        Mode::Dark => gpui_component::ThemeMode::Dark,
    };
    gpui_component::Theme::change(mode, Some(window), cx);
    // Edited through `global_mut` so the clear stays on the root's token: inputs in
    // light mode take their fill from the solid `background` colour, which keeps.
    gpui_component::Theme::global_mut(cx).tokens.background = gpui::transparent_black().into();
}

/// Whether the system asks apps to reduce motion (macOS "Reduce motion"). Linux has no
/// common setting, so it is never set there.
pub fn reduce_motion() -> bool {
    #[cfg(target_os = "macos")]
    {
        objc2_app_kit::NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// Whether the window draws without its buttons and the header strip that holds them.
/// On macOS the buttons are never shown (remove-window-buttons); quitting, minimising
/// and hiding are keys instead.
pub fn buttonless(appearance: &Appearance) -> bool {
    cfg!(target_os = "macos") || appearance.decorations != Decorations::Full
}

/// Applies the window-level settings: background translucency and blur, and the
/// window buttons.
pub fn apply_to_window(appearance: &Appearance, window: &mut Window) {
    let background = window_background(appearance);
    if background == WindowBackgroundAppearance::Blurred && cfg!(not(target_os = "macos")) {
        // GPUI blurs only where the compositor offers it (some Wayland compositors);
        // elsewhere the window is translucent without blur. Said once per run.
        static TOLD: std::sync::Once = std::sync::Once::new();
        TOLD.call_once(|| {
            eprintln!(
                "headquarter-app: blur depends on the compositor; without support the window is translucent only"
            );
        });
    }
    window.set_background_appearance(background);
    window_buttons::set_hidden(window, buttonless(appearance));
    #[cfg(target_os = "linux")]
    window.request_decorations(if appearance.decorations == Decorations::None {
        gpui::WindowDecorations::Client
    } else {
        gpui::WindowDecorations::Server
    });
}

#[cfg(target_os = "macos")]
mod window_buttons {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSApplication, NSWindowButton};

    /// Hides or shows the close, minimise and zoom buttons; GPUI can only move them.
    /// The app has one window, so this applies to every window it owns.
    pub fn set_hidden(_window: &gpui::Window, hidden: bool) {
        // GPUI calls into the app on the main thread; elsewhere there is nothing to do.
        let Some(main_thread) = MainThreadMarker::new() else {
            return;
        };
        for ns_window in NSApplication::sharedApplication(main_thread).windows() {
            for kind in [
                NSWindowButton::CloseButton,
                NSWindowButton::MiniaturizeButton,
                NSWindowButton::ZoomButton,
            ] {
                if let Some(button) = ns_window.standardWindowButton(kind) {
                    button.setHidden(hidden);
                }
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod window_buttons {
    /// Window buttons belong to the window manager here; `decorations` handles them.
    pub fn set_hidden(_window: &gpui::Window, _hidden: bool) {}
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::terminal::keys::{self, KeyModes, KeyPress};
    use crate::theme::Theme;

    fn owner_settings() -> Appearance {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../hq-config/tests/fixtures");
        let source = std::fs::read_to_string(fixtures.join("alacritty.toml")).unwrap();
        let home = fixtures.join("home");
        hq_config::parse(
            &source,
            &home.join(".config/headquarter/config.toml"),
            &["claude", "codex", "agy"],
            Some(&home),
        )
        .unwrap()
        .appearance
    }

    /// JetBrainsMono Nerd Font's faces as Core Text lists them on the owner's Mac.
    fn jetbrains(family: &str, style: &str) -> Option<bool> {
        let faces = [
            "Regular",
            "Italic",
            "Medium",
            "Medium Italic",
            "Bold",
            "ExtraBold",
        ];
        (family == "JetBrainsMono Nerd Font").then(|| faces.contains(&style))
    }

    fn unknown(_family: &str, _style: &str) -> Option<bool> {
        None
    }

    #[test]
    fn styles_the_font_lacks_use_the_normal_face_like_alacritty() {
        // The owner's file: bold "Heavy", italic "Italic", bold italic "Heavy Italic".
        let appearance = owner_settings();
        let options = pane_options_with(&appearance, &jetbrains);
        assert_eq!(options.bold_weight, FontWeight::MEDIUM, "no Heavy face");
        assert_eq!(options.bold_italic_weight, FontWeight::MEDIUM);
        assert!(!options.bold_italic_slanted, "Alacritty draws it upright");
        assert_eq!(options.italic_weight, FontWeight::NORMAL, "Italic exists");
        assert!(options.italic_slanted);
        assert_eq!(
            font_notes(&appearance, &jetbrains),
            [
                "font.bold style \"Heavy\" is not in JetBrainsMono Nerd Font; bold text uses the normal face, as in Alacritty",
                "font.bold_italic style \"Heavy Italic\" is not in JetBrainsMono Nerd Font; bold italic text uses the normal face, as in Alacritty",
            ]
        );
    }

    #[test]
    fn styles_the_font_has_keep_their_face() {
        let mut appearance = owner_settings();
        appearance.font.bold_style = Some("ExtraBold".into());
        appearance.font.bold_weight = Some(800);
        let options = pane_options_with(&appearance, &jetbrains);
        assert_eq!(options.bold_weight, FontWeight::EXTRA_BOLD);
        assert!(
            !font_notes(&appearance, &jetbrains)
                .iter()
                .any(|note| note.starts_with("font.bold ")),
            "no note for a face the font has"
        );
        let unchecked = pane_options_with(&owner_settings(), &unknown);
        assert_eq!(
            unchecked.bold_weight,
            FontWeight::BLACK,
            "nothing changes where faces cannot be listed (Linux, unknown family)"
        );
        assert!(font_notes(&owner_settings(), &unknown).is_empty());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn core_text_lists_the_faces_of_a_family() {
        // Menlo ships with macOS.
        assert_eq!(has_face("Menlo", "Bold"), Some(true));
        assert_eq!(has_face("Menlo", "Heavy"), Some(false));
        assert_eq!(
            has_face("Menlo", "bold"),
            Some(false),
            "exact names, as Alacritty"
        );
    }

    #[test]
    fn owner_settings_become_pane_options() {
        let options = pane_options_with(&owner_settings(), &unknown);
        assert_eq!(options.font_family.as_ref(), "JetBrainsMono Nerd Font");
        assert_eq!(options.font_size, 14.0);
        assert_eq!(options.normal_weight, FontWeight::MEDIUM);
        assert_eq!(options.bold_weight, FontWeight::BLACK);
        assert_eq!(options.italic_weight, FontWeight::NORMAL);
        assert_eq!(options.padding, (10.0, 10.0));
        assert_eq!(options.cursor, Some(CursorShape::Beam));
        assert!((options.opacity - 0.7).abs() < f32::EPSILON);
        assert!(options.alt_is_meta);
    }

    #[test]
    fn owner_shift_enter_sends_a_newline_to_the_agent() {
        let options = pane_options(&owner_settings());
        let modes = KeyModes {
            alt_is_meta: options.alt_is_meta,
            ..KeyModes::default()
        };
        let shift_enter = KeyPress {
            key: "enter",
            shift: true,
            ..KeyPress::default()
        };
        assert_eq!(
            keys::encode(shift_enter, modes, &options.bindings).unwrap(),
            b"\x1b\r"
        );
        let enter = KeyPress {
            key: "enter",
            ..KeyPress::default()
        };
        assert_eq!(
            keys::encode(enter, modes, &options.bindings).unwrap(),
            b"\r"
        );
        let option_b = KeyPress {
            key: "b",
            alt: true,
            ..KeyPress::default()
        };
        assert_eq!(
            keys::encode(option_b, modes, &options.bindings).unwrap(),
            b"\x1bb",
            "Option acts as Alt with option_as_alt = \"Both\""
        );
    }

    #[test]
    fn option_as_alt_sides() {
        for (setting, meta) in [
            (OptionAsAlt::None, false),
            (OptionAsAlt::Left, true),
            (OptionAsAlt::Right, true),
            (OptionAsAlt::Both, true),
        ] {
            let appearance = Appearance {
                option_as_alt: setting,
                ..Appearance::default()
            };
            assert_eq!(pane_options(&appearance).alt_is_meta, meta, "{setting:?}");
        }
    }

    #[test]
    fn window_background_follows_opacity_and_blur() {
        let owner = owner_settings();
        assert_eq!(
            window_background(&owner),
            WindowBackgroundAppearance::Blurred
        );
        assert!(buttonless(&owner));
        let clear = Appearance {
            opacity: 0.8,
            ..Appearance::default()
        };
        assert_eq!(
            window_background(&clear),
            WindowBackgroundAppearance::Transparent
        );
        let opaque = Appearance {
            blur: true,
            ..Appearance::default()
        };
        assert_eq!(
            window_background(&opaque),
            WindowBackgroundAppearance::Opaque
        );
        assert_eq!(
            buttonless(&Appearance::default()),
            cfg!(target_os = "macos"),
            "full decorations show the buttons everywhere but macOS"
        );
    }

    #[test]
    fn owner_theme_uses_their_colours() {
        let theme = Theme::resolve(Mode::Light, &owner_settings().colors);
        assert_eq!(theme.mode, Mode::Dark);
        assert_eq!(theme.palette.background, crate::theme::rgb(0x1E1E2E));
    }

    #[test]
    fn invalid_settings_keep_the_last_good_ones() {
        let dir = std::env::temp_dir().join(format!("hq-app-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "[window]\nopacity = 0.5\n").unwrap();
        let mut watcher = SettingsWatcher::new(Some(path.clone()), vec!["claude"]);
        let (good, note) = read(&mut watcher, None);
        assert_eq!(good.opacity, 0.5);
        assert_eq!(note, None);
        std::fs::write(&path, "[window]\nopacity = 7\n").unwrap();
        let (kept, note) = read(&mut watcher, Some(&good));
        assert_eq!(kept.opacity, 0.5);
        assert!(note.unwrap().contains("opacity must be between"));
        std::fs::write(&path, "[env]\nTERM = \"x\"\n").unwrap();
        let (_, note) = read(&mut watcher, Some(&good));
        assert!(note.unwrap().contains("`env`"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
