//! Theme files: the owner's themes in `themes/` next to the settings file, and the
//! templates shipped with the app (`openspec/changes/add-theme-creator/design.md`, D5).
//!
//! Each of the owner's themes is two files. `<name>.toml` holds only what Alacritty
//! reads, so Alacritty imports it with no warning:
//!
//! ```toml
//! [colors.primary]
//! background = "#1e1e2e"
//! foreground = "#cdd6f4"
//! [window]
//! opacity = 0.9
//! blur = true
//! ```
//!
//! `<name>.headquarter.toml` holds the rest:
//!
//! ```toml
//! [headquarter]
//! name = "Mocha orange"
//! [headquarter.colors]
//! accent = "#fab387"
//! [headquarter.opacity]
//! sidebar = 80
//! ```
//!
//! A colour file alone is a theme named after the file, so any Alacritty theme can be
//! dropped in. The templates keep both parts in one file. Unlike the settings file,
//! unknown keys are ignored, so files from other tools load.

use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::ColorSettings;
use crate::appearance::imported_colors;

/// The interface colours a file may set, by their names in `[headquarter.colors]`.
pub const INTERFACE_COLOURS: [&str; 10] = [
    "sidebar",
    "raised",
    "lines",
    "tracks",
    "text",
    "muted",
    "groups",
    "accent",
    "needs_you",
    "needs_you_icon",
];

/// The terminal colours' names in `[headquarter.opacity]`, in palette order after the
/// four primary ones.
pub const TERMINAL_NAMES: [&str; 8] = [
    "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
];

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ThemeFile {
    pub name: String,
    /// Shown beside a template's name, such as "Dracula · MIT".
    pub credit: Option<String>,
    /// The terminal's colours, as Alacritty reads them.
    pub colors: ColorSettings,
    /// Interface colours by their `INTERFACE_COLOURS` name, as `0xRRGGBB`.
    pub interface: BTreeMap<String, u32>,
    /// Opacity in percent by colour name, for colours that are not fully opaque.
    pub opacity: BTreeMap<String, u8>,
    pub window_opacity: Option<f32>,
    pub window_blur: Option<bool>,
}

/// One theme in the themes folder, by its colour file: the theme, or why it cannot be
/// read.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    pub file: String,
    pub theme: Result<ThemeFile, String>,
}

/// The ending of a companion file, beside `<name>.toml`.
const COMPANION: &str = ".headquarter.toml";

/// The companion's name for a colour file.
pub fn companion_of(file: &str) -> String {
    format!("{}{COMPANION}", file.trim_end_matches(".toml"))
}

/// The folder for the owner's themes, beside the settings file.
pub fn themes_dir() -> Option<PathBuf> {
    crate::settings_path().and_then(|path| path.parent().map(|dir| dir.join("themes")))
}

/// A theme from one text holding Alacritty's colours and window settings and,
/// optionally, the `[headquarter]` table. A theme with no name gets an empty one.
pub fn parse(text: &str) -> Result<ThemeFile, String> {
    let table: toml::Table =
        toml::from_str(text).map_err(|error| error.message().trim().to_owned())?;
    let colors = imported_colors(text)?;
    let headquarter = table.get("headquarter").and_then(toml::Value::as_table);
    let name = headquarter
        .and_then(|table| table.get("name"))
        .and_then(toml::Value::as_str)
        .map(str::trim)
        .unwrap_or_default()
        .to_owned();
    let credit = headquarter
        .and_then(|table| table.get("credit"))
        .and_then(toml::Value::as_str)
        .map(str::to_owned);
    let mut interface = BTreeMap::new();
    if let Some(table) = headquarter
        .and_then(|table| table.get("colors"))
        .and_then(toml::Value::as_table)
    {
        for (key, value) in table {
            if !INTERFACE_COLOURS.contains(&key.as_str()) {
                continue;
            }
            let text = value.as_str().ok_or(format!("`{key}` is not a colour"))?;
            interface.insert(
                key.clone(),
                hex(text).ok_or(format!("`{text}` is not a colour"))?,
            );
        }
    }
    let mut opacity = BTreeMap::new();
    if let Some(table) = headquarter
        .and_then(|table| table.get("opacity"))
        .and_then(toml::Value::as_table)
    {
        for (key, value) in table {
            let percent = value
                .as_integer()
                .and_then(|percent| u8::try_from(percent).ok())
                .filter(|percent| *percent <= 100)
                .ok_or(format!("the opacity of `{key}` is not 0 to 100"))?;
            opacity.insert(key.clone(), percent);
        }
    }
    let window = table.get("window").and_then(toml::Value::as_table);
    let window_opacity = window
        .and_then(|window| window.get("opacity"))
        .and_then(|value| {
            value
                .as_float()
                .or_else(|| value.as_integer().map(|whole| whole as f64))
        })
        .map(|opacity| opacity.clamp(0.0, 1.0) as f32);
    let window_blur = window
        .and_then(|window| window.get("blur"))
        .and_then(toml::Value::as_bool);
    Ok(ThemeFile {
        name,
        credit,
        colors,
        interface,
        opacity,
        window_opacity,
        window_blur,
    })
}

fn hex(text: &str) -> Option<u32> {
    let text = text.trim();
    let digits = text
        .strip_prefix('#')
        .or_else(|| text.strip_prefix("0x"))
        .unwrap_or(text);
    (digits.len() == 6)
        .then(|| u32::from_str_radix(digits, 16).ok())
        .flatten()
}

/// The colour file's text: only what Alacritty reads. Colours are written `#rrggbb`;
/// opacity is in the companion because Alacritty rejects 8-digit colours.
pub fn render_colours(theme: &ThemeFile, companion: &str) -> String {
    let colour = |value: u32| format!("\"#{value:06x}\"");
    let mut out = format!(
        "# Headquarter theme \"{}\": Alacritty can import this file.\n# Its interface colours are in {companion}.\n",
        theme.name.replace('"', "'")
    );
    let colors = &theme.colors;
    let mut primary = String::new();
    for (key, value) in [
        ("background", colors.background),
        ("foreground", colors.foreground),
    ] {
        if let Some(value) = value {
            primary.push_str(&format!("{key} = {}\n", colour(value)));
        }
    }
    if !primary.is_empty() {
        out.push_str(&format!("\n[colors.primary]\n{primary}"));
    }
    if let Some(cursor) = colors.cursor {
        out.push_str(&format!("\n[colors.cursor]\ncursor = {}\n", colour(cursor)));
    }
    if let Some(selection) = colors.selection {
        out.push_str(&format!(
            "\n[colors.selection]\nbackground = {}\n",
            colour(selection)
        ));
    }
    for (table, eight) in [("normal", &colors.normal), ("bright", &colors.bright)] {
        if eight.iter().any(Option::is_some) {
            out.push_str(&format!("\n[colors.{table}]\n"));
            for (name, value) in TERMINAL_NAMES.iter().zip(eight) {
                if let Some(value) = value {
                    out.push_str(&format!("{name} = {}\n", colour(*value)));
                }
            }
        }
    }
    if theme.window_opacity.is_some() || theme.window_blur.is_some() {
        out.push_str("\n[window]\n");
        if let Some(opacity) = theme.window_opacity {
            out.push_str(&format!("opacity = {:.2}\n", opacity));
        }
        if let Some(blur) = theme.window_blur {
            out.push_str(&format!("blur = {blur}\n"));
        }
    }
    out
}

/// The companion's text: the name, the interface colours and the opacities.
pub fn render_companion(theme: &ThemeFile) -> String {
    let colour = |value: u32| format!("\"#{value:06x}\"");
    let quoted = |text: &str| toml::Value::String(text.to_owned()).to_string();
    let mut out = format!(
        "# Headquarter's part of the theme; the colours Alacritty reads are beside it.\n\n[headquarter]\nname = {}\n",
        quoted(&theme.name)
    );
    if let Some(credit) = &theme.credit {
        out.push_str(&format!("credit = {}\n", quoted(credit)));
    }
    if !theme.interface.is_empty() {
        out.push_str("\n[headquarter.colors]\n");
        for (key, value) in &theme.interface {
            out.push_str(&format!("{key} = {}\n", colour(*value)));
        }
    }
    if !theme.opacity.is_empty() {
        out.push_str("\n[headquarter.opacity]\n");
        for (key, value) in &theme.opacity {
            out.push_str(&format!("{} = {value}\n", quoted_key(key)));
        }
    }
    out
}

/// A theme from its colour file's text and its companion's, if there is one; with no
/// name, it is named after the file.
pub fn parse_pair(colours: &str, companion: Option<&str>, file: &str) -> Result<ThemeFile, String> {
    let mut theme = parse(colours)?;
    if let Some(companion) = companion {
        let extra = parse(companion).map_err(|error| format!("{}: {error}", companion_of(file)))?;
        theme.name = extra.name;
        theme.credit = extra.credit;
        theme.interface = extra.interface;
        theme.opacity = extra.opacity;
    }
    if theme.name.is_empty() {
        theme.name = file.trim_end_matches(".toml").to_owned();
    }
    Ok(theme)
}

/// Opacity keys such as `normal.red` need quotes to stay one key.
fn quoted_key(key: &str) -> String {
    if key.contains('.') {
        format!("\"{key}\"")
    } else {
        key.to_owned()
    }
}

/// Every theme in `dir` by its colour file, sorted by name; companions are read with
/// their colour file, and one without a colour file is ignored. A missing folder lists
/// nothing.
pub fn list(dir: &Path) -> Vec<Listed> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut listed: Vec<Listed> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file = entry.file_name().into_string().ok()?;
            if !file.ends_with(".toml") || file.ends_with(COMPANION) || file.starts_with('.') {
                return None;
            }
            let companion = std::fs::read_to_string(dir.join(companion_of(&file))).ok();
            let theme = std::fs::read_to_string(entry.path())
                .map_err(|error| error.to_string())
                .and_then(|text| parse_pair(&text, companion.as_deref(), &file));
            Some(Listed { file, theme })
        })
        .collect();
    listed.sort_by_key(|listed| {
        (
            listed.theme.as_ref().map_or_else(
                |_| listed.file.to_lowercase(),
                |theme| theme.name.to_lowercase(),
            ),
            listed.file.clone(),
        )
    });
    listed
}

/// A file name for a theme called `name`: lower case, dashes, and `-2`, `-3` when the
/// name is taken.
pub fn file_name_for(name: &str, taken: &[String]) -> String {
    let mut stem = String::new();
    for character in name.trim().to_lowercase().chars() {
        if character.is_ascii_alphanumeric() {
            stem.push(character);
        } else if !stem.ends_with('-') && !stem.is_empty() {
            stem.push('-');
        }
    }
    let stem = stem.trim_end_matches('-');
    let stem = if stem.is_empty() { "theme" } else { stem };
    let mut file = format!("{stem}.toml");
    let mut number = 2;
    while taken.contains(&file) {
        file = format!("{stem}-{number}.toml");
        number += 1;
    }
    file
}

/// Writes the theme's colour file and companion, each whole (a temporary file, then a
/// rename). `replacing` keeps an edited theme's file name; a new theme gets one from its
/// name. Returns the colour file's name.
pub fn save(dir: &Path, theme: &ThemeFile, replacing: Option<&str>) -> io::Result<String> {
    std::fs::create_dir_all(dir)?;
    let file = match replacing {
        Some(file) => file.to_owned(),
        None => {
            let taken: Vec<String> = list(dir).into_iter().map(|listed| listed.file).collect();
            file_name_for(&theme.name, &taken)
        }
    };
    let companion = companion_of(&file);
    write_whole(dir, &file, &render_colours(theme, &companion))?;
    write_whole(dir, &companion, &render_companion(theme))?;
    Ok(file)
}

fn write_whole(dir: &Path, file: &str, text: &str) -> io::Result<()> {
    let partial = dir.join(format!(".{file}.part"));
    std::fs::write(&partial, text)?;
    std::fs::rename(&partial, dir.join(file))
}

/// Removes a theme's colour file and its companion, if it has one.
pub fn delete(dir: &Path, file: &str) -> io::Result<()> {
    std::fs::remove_file(dir.join(file))?;
    match std::fs::remove_file(dir.join(companion_of(file))) {
        Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("hq-themes-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn sample() -> ThemeFile {
        let mut colors = ColorSettings {
            background: Some(0x1e1e2e),
            foreground: Some(0xcdd6f4),
            cursor: Some(0xf5e0dc),
            ..ColorSettings::default()
        };
        colors.normal[1] = Some(0xf38ba8);
        colors.bright[7] = Some(0xa6adc8);
        ThemeFile {
            name: "Mocha \"orange\"".to_owned(),
            credit: None,
            colors,
            interface: BTreeMap::from([("accent".to_owned(), 0xfab387)]),
            opacity: BTreeMap::from([("sidebar".to_owned(), 80), ("normal.red".to_owned(), 60)]),
            window_opacity: Some(0.9),
            window_blur: Some(true),
        }
    }

    /// Writes both files and reads them back the way the folder is listed.
    fn round_trip(theme: &ThemeFile) -> ThemeFile {
        let companion = companion_of("x.toml");
        parse_pair(
            &render_colours(theme, &companion),
            Some(&render_companion(theme)),
            "x.toml",
        )
        .unwrap()
    }

    #[test]
    fn a_theme_round_trips_through_its_two_files() {
        let theme = sample();
        assert_eq!(round_trip(&theme), theme);
    }

    #[test]
    fn the_colour_file_is_only_what_alacritty_reads() {
        let text = render_colours(&sample(), "mocha-orange.headquarter.toml");
        assert_eq!(imported_colors(&text).unwrap(), sample().colors);
        let table: toml::Table = toml::from_str(&text).unwrap();
        let keys: Vec<&str> = table.keys().map(String::as_str).collect();
        assert_eq!(
            keys,
            ["colors", "window"],
            "no table Alacritty would warn about"
        );
        for line in text.lines() {
            if let Some(start) = line.find("\"#") {
                assert_eq!(line[start + 2..].find('"'), Some(6), "{line}");
            }
        }
    }

    #[test]
    fn unknown_keys_are_ignored_and_errors_are_reported() {
        let theme = parse("[headquarter]\nname = \"x\"\nfuture = 1\n[headquarter.colors]\nglow = \"#000000\"\n[shell]\nprogram = \"zsh\"\n").unwrap();
        assert_eq!(theme.name, "x");
        assert!(theme.interface.is_empty());
        assert!(parse("not toml = = 1").is_err());
        assert!(
            parse("[headquarter]\nname = \"x\"\n[headquarter.opacity]\nsidebar = 300\n")
                .unwrap_err()
                .contains("0 to 100")
        );
        let broken_companion = parse_pair("", Some("[[["), "nord.toml").unwrap_err();
        assert!(
            broken_companion.starts_with("nord.headquarter.toml"),
            "{broken_companion}"
        );
    }

    #[test]
    fn an_alacritty_theme_alone_is_named_after_its_file() {
        let theme = parse_pair(
            "[colors.primary]\nbackground = \"#282828\"\n[colors.search.matches]\nforeground = \"#000000\"\n",
            None,
            "gruvbox_material.toml",
        )
        .unwrap();
        assert_eq!(theme.name, "gruvbox_material");
        assert_eq!(theme.colors.background, Some(0x282828));
        assert!(theme.interface.is_empty());
    }

    #[test]
    fn files_are_saved_and_deleted_in_pairs() {
        let dir = scratch("save");
        let first = save(&dir, &sample(), None).unwrap();
        assert_eq!(first, "mocha-orange.toml");
        assert!(dir.join("mocha-orange.headquarter.toml").exists());
        let second = save(&dir, &sample(), None).unwrap();
        assert_eq!(second, "mocha-orange-2.toml", "a taken name gets a number");
        let mut edited = sample();
        edited.name = "Edited".to_owned();
        assert_eq!(save(&dir, &edited, Some(&first)).unwrap(), first);
        std::fs::write(dir.join("broken.toml"), "[[[").unwrap();
        std::fs::write(dir.join("notes.txt"), "not a theme").unwrap();
        std::fs::write(
            dir.join("orphan.headquarter.toml"),
            "[headquarter]\nname = \"o\"\n",
        )
        .unwrap();
        let listed = list(&dir);
        let names: Vec<_> = listed.iter().map(|listed| listed.file.as_str()).collect();
        assert_eq!(
            names,
            ["broken.toml", "mocha-orange.toml", "mocha-orange-2.toml"],
            "companions are read with their colour file; an orphan is ignored"
        );
        assert!(
            listed[0].theme.is_err(),
            "a broken file is listed with its error"
        );
        assert_eq!(listed[1].theme.as_ref().unwrap().name, "Edited");
        assert!(!dir.join(".mocha-orange.toml.part").exists());
        delete(&dir, &second).unwrap();
        assert!(!dir.join("mocha-orange-2.headquarter.toml").exists());
        assert!(
            dir.join("orphan.headquarter.toml").exists(),
            "never deleted"
        );
        assert_eq!(list(&dir).len(), 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_names() {
        assert_eq!(file_name_for("  Nord · Orange!! ", &[]), "nord-orange.toml");
        assert_eq!(file_name_for("···", &[]), "theme.toml");
        assert_eq!(
            file_name_for("a", &["a.toml".to_owned(), "a-2.toml".to_owned()]),
            "a-3.toml"
        );
    }

    #[test]
    fn a_missing_folder_lists_nothing() {
        assert!(list(Path::new("/nonexistent/hq/themes")).is_empty());
    }
}
