//! Appearance and keyboard settings, named as in Alacritty so an Alacritty setup
//! carries over.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::{Problem, problem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decorations {
    Full,
    /// No window buttons and no header strip above the terminal.
    Buttonless,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionAsAlt {
    None,
    Left,
    Right,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
    Block,
    Beam,
    Underline,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FontSettings {
    pub family: Option<String>,
    pub size: Option<f32>,
    pub normal_weight: Option<u16>,
    pub bold_weight: Option<u16>,
    pub italic_weight: Option<u16>,
    pub bold_italic_weight: Option<u16>,
}

/// Colours as `0xRRGGBB`. `None` keeps the built-in theme's colour.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ColorSettings {
    pub foreground: Option<u32>,
    pub background: Option<u32>,
    pub cursor: Option<u32>,
    pub selection: Option<u32>,
    pub normal: [Option<u32>; 8],
    pub bright: [Option<u32>; 8],
    pub dim: [Option<u32>; 8],
}

impl ColorSettings {
    pub fn is_empty(&self) -> bool {
        *self == ColorSettings::default()
    }

    fn merge(&mut self, over: &ColorSettings) {
        let pick = |base: &mut Option<u32>, new: Option<u32>| {
            if new.is_some() {
                *base = new;
            }
        };
        pick(&mut self.foreground, over.foreground);
        pick(&mut self.background, over.background);
        pick(&mut self.cursor, over.cursor);
        pick(&mut self.selection, over.selection);
        for index in 0..8 {
            pick(&mut self.normal[index], over.normal[index]);
            pick(&mut self.bright[index], over.bright[index]);
            pick(&mut self.dim[index], over.dim[index]);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBinding {
    /// Key name as the app's toolkit names it: "enter", "a", "up", "f5".
    pub key: String,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub platform: bool,
    pub chars: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Appearance {
    pub opacity: f32,
    pub blur: bool,
    pub padding: (u16, u16),
    pub decorations: Decorations,
    pub option_as_alt: OptionAsAlt,
    pub dim_unfocused: f32,
    pub font: FontSettings,
    pub cursor: Option<CursorShape>,
    pub colors: ColorSettings,
    pub bindings: Vec<KeyBinding>,
    /// Whether edits to the settings file apply while the app runs.
    pub live_reload: bool,
    pub agent_marks: AgentMarks,
}

/// How agents are marked in the sidebar, the session modal and the usage footer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMarks {
    /// Each vendor's own one-colour mark where it publishes one, drawn in black or
    /// white; its coloured mark otherwise.
    Mono,
    /// Each vendor's own mark in its colours.
    Color,
    /// Headquarter's own drawn marks; nothing is fetched.
    Drawn,
}

impl Default for Appearance {
    fn default() -> Self {
        Appearance {
            opacity: 1.0,
            blur: false,
            padding: (12, 10),
            decorations: Decorations::Full,
            option_as_alt: OptionAsAlt::None,
            dim_unfocused: 0.0,
            font: FontSettings::default(),
            cursor: None,
            colors: ColorSettings::default(),
            bindings: Vec::new(),
            live_reload: true,
            agent_marks: AgentMarks::Mono,
        }
    }
}

#[derive(Deserialize, Default)]
pub(crate) struct RawGeneral {
    #[serde(default)]
    import: Vec<toml::Spanned<String>>,
    live_config_reload: Option<bool>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawWindow {
    opacity: Option<toml::Spanned<f64>>,
    blur: Option<bool>,
    padding: Option<RawPadding>,
    decorations: Option<toml::Spanned<String>>,
    option_as_alt: Option<toml::Spanned<String>>,
    dim_unfocused: Option<toml::Spanned<f64>>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawSidebar {
    agent_marks: Option<toml::Spanned<String>>,
}

#[derive(Deserialize, Default)]
struct RawPadding {
    x: Option<toml::Spanned<i64>>,
    y: Option<toml::Spanned<i64>>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawFont {
    size: Option<toml::Spanned<f64>>,
    normal: Option<RawFace>,
    bold: Option<RawFace>,
    italic: Option<RawFace>,
    bold_italic: Option<RawFace>,
}

#[derive(Deserialize, Default)]
struct RawFace {
    family: Option<toml::Spanned<String>>,
    style: Option<String>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawCursor {
    /// A shape name, or a table with a `shape` (Alacritty takes both).
    style: Option<toml::Spanned<toml::Value>>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawColors {
    #[serde(default)]
    primary: RawPrimary,
    #[serde(default)]
    normal: RawEight,
    #[serde(default)]
    bright: RawEight,
    #[serde(default)]
    dim: RawEight,
    #[serde(default)]
    cursor: RawCursorColors,
    #[serde(default)]
    selection: RawSelectionColors,
}

#[derive(Deserialize, Default)]
struct RawPrimary {
    foreground: Option<toml::Spanned<String>>,
    background: Option<toml::Spanned<String>>,
}

#[derive(Deserialize, Default)]
struct RawEight {
    black: Option<toml::Spanned<String>>,
    red: Option<toml::Spanned<String>>,
    green: Option<toml::Spanned<String>>,
    yellow: Option<toml::Spanned<String>>,
    blue: Option<toml::Spanned<String>>,
    magenta: Option<toml::Spanned<String>>,
    cyan: Option<toml::Spanned<String>>,
    white: Option<toml::Spanned<String>>,
}

#[derive(Deserialize, Default)]
struct RawCursorColors {
    cursor: Option<toml::Spanned<String>>,
}

#[derive(Deserialize, Default)]
struct RawSelectionColors {
    background: Option<toml::Spanned<String>>,
}

#[derive(Deserialize, Default)]
pub(crate) struct RawKeyboard {
    #[serde(default)]
    bindings: Vec<RawBinding>,
}

#[derive(Deserialize)]
struct RawBinding {
    key: toml::Spanned<String>,
    mods: Option<String>,
    chars: Option<String>,
}

pub(crate) struct RawAppearance {
    pub general: RawGeneral,
    pub window: RawWindow,
    pub font: RawFont,
    pub cursor: RawCursor,
    pub colors: RawColors,
    pub keyboard: RawKeyboard,
    pub sidebar: RawSidebar,
}

/// Builds the appearance. Imported colour files are read relative to `config_dir`.
/// Warnings are for Alacritty features Headquarter does not offer.
pub(crate) fn build(
    raw: RawAppearance,
    config_dir: &Path,
    home: Option<&Path>,
    warnings: &mut Vec<Problem>,
) -> Result<Appearance, Problem> {
    let mut appearance = Appearance {
        live_reload: raw.general.live_config_reload.unwrap_or(true),
        ..Appearance::default()
    };

    if let Some(opacity) = raw.window.opacity {
        appearance.opacity = in_range(&opacity, 0.0, 1.0, "opacity")?;
    }
    appearance.blur = raw.window.blur.unwrap_or(false);
    if let Some(padding) = raw.window.padding {
        let side = |value: Option<toml::Spanned<i64>>, default: u16| -> Result<u16, Problem> {
            match value {
                None => Ok(default),
                Some(value) => u16::try_from(*value.get_ref())
                    .ok()
                    .filter(|pixels| *pixels <= 200)
                    .ok_or_else(|| {
                        problem(value.span().start, "padding must be between 0 and 200")
                    }),
            }
        };
        appearance.padding = (side(padding.x, 12)?, side(padding.y, 10)?);
    }
    if let Some(decorations) = raw.window.decorations {
        appearance.decorations = match decorations.get_ref().to_ascii_lowercase().as_str() {
            "full" => Decorations::Full,
            "buttonless" => Decorations::Buttonless,
            "none" => Decorations::None,
            "transparent" => {
                warnings.push(problem(
                    decorations.span().start,
                    "decorations = \"Transparent\" is not supported; using \"Buttonless\"",
                ));
                Decorations::Buttonless
            }
            _ => {
                return Err(problem(
                    decorations.span().start,
                    "decorations must be \"Full\", \"Buttonless\" or \"None\"",
                ));
            }
        };
    }
    if let Some(option) = raw.window.option_as_alt {
        appearance.option_as_alt = match option.get_ref().to_ascii_lowercase().as_str() {
            "none" => OptionAsAlt::None,
            "onlyleft" | "left" => OptionAsAlt::Left,
            "onlyright" | "right" => OptionAsAlt::Right,
            "both" => OptionAsAlt::Both,
            _ => {
                return Err(problem(
                    option.span().start,
                    "option_as_alt must be \"None\", \"OnlyLeft\", \"OnlyRight\" or \"Both\"",
                ));
            }
        };
    }
    if let Some(dim) = raw.window.dim_unfocused {
        appearance.dim_unfocused = in_range(&dim, 0.0, 0.6, "dim_unfocused")?;
    }
    if let Some(marks) = raw.sidebar.agent_marks {
        appearance.agent_marks = match marks.get_ref().to_ascii_lowercase().as_str() {
            "mono" => AgentMarks::Mono,
            "color" => AgentMarks::Color,
            "drawn" => AgentMarks::Drawn,
            _ => {
                return Err(problem(
                    marks.span().start,
                    "agent_marks must be \"mono\", \"color\" or \"drawn\"",
                ));
            }
        };
    }

    if let Some(size) = raw.font.size {
        appearance.font.size = Some(in_range(&size, 6.0, 72.0, "font size")?);
    }
    if let Some(normal) = raw.font.normal {
        appearance.font.family = normal.family.map(toml::Spanned::into_inner);
        appearance.font.normal_weight = normal.style.as_deref().map(weight_of);
    }
    let faces = [
        (raw.font.bold, &mut appearance.font.bold_weight),
        (raw.font.italic, &mut appearance.font.italic_weight),
        (
            raw.font.bold_italic,
            &mut appearance.font.bold_italic_weight,
        ),
    ];
    for (face, weight) in faces {
        let Some(face) = face else {
            continue;
        };
        *weight = face.style.as_deref().map(weight_of);
        // One family is used for every style; a different one is reported, not used.
        if let Some(family) = face.family
            && Some(family.get_ref()) != appearance.font.family.as_ref()
        {
            warnings.push(problem(
                family.span().start,
                "a different font family per style is not supported; the normal family is used",
            ));
        }
    }

    if let Some(style) = raw.cursor.style {
        let shape = match style.get_ref() {
            toml::Value::Table(table) => table.get("shape"),
            value => Some(value),
        };
        if let Some(shape) = shape {
            let name = shape.as_str().map(str::to_ascii_lowercase);
            appearance.cursor = Some(match name.as_deref() {
                Some("block") => CursorShape::Block,
                Some("beam") => CursorShape::Beam,
                Some("underline") => CursorShape::Underline,
                _ => {
                    return Err(problem(
                        style.span().start,
                        "cursor style must be \"Block\", \"Beam\" or \"Underline\"",
                    ));
                }
            });
        }
    }

    for import in &raw.general.import {
        let path = expand(import.get_ref(), config_dir, home);
        let text = std::fs::read_to_string(&path).map_err(|error| {
            problem(
                import.span().start,
                format!("cannot read imported file {}: {error}", path.display()),
            )
        })?;
        let imported = imported_colors(&text).map_err(|message| {
            problem(
                import.span().start,
                format!("imported file {}: {message}", path.display()),
            )
        })?;
        appearance.colors.merge(&imported);
    }
    let own = colors(&raw.colors).map_err(|(offset, message)| problem(offset, message))?;
    appearance.colors.merge(&own);

    for binding in raw.keyboard.bindings {
        let Some(chars) = binding.chars else {
            warnings.push(problem(
                binding.key.span().start,
                "key bindings to actions are not supported; only `chars` bindings are",
            ));
            continue;
        };
        let key = key_name(binding.key.get_ref()).ok_or_else(|| {
            problem(
                binding.key.span().start,
                format!("unknown key `{}`", binding.key.get_ref()),
            )
        })?;
        let mods = binding.mods.unwrap_or_default();
        let has = |names: &[&str]| {
            mods.split('|')
                .map(str::trim)
                .any(|part| names.iter().any(|name| part.eq_ignore_ascii_case(name)))
        };
        appearance.bindings.push(KeyBinding {
            key,
            control: has(&["Control", "Ctrl"]),
            alt: has(&["Alt", "Option"]),
            shift: has(&["Shift"]),
            platform: has(&["Super", "Command", "Cmd"]),
            chars: chars.into_bytes(),
        });
    }

    Ok(appearance)
}

/// The imported files, resolved as `build` reads them.
pub(crate) fn imports(
    general: &RawGeneral,
    config_dir: &Path,
    home: Option<&Path>,
) -> Vec<PathBuf> {
    general
        .import
        .iter()
        .map(|import| expand(import.get_ref(), config_dir, home))
        .collect()
}

fn in_range(value: &toml::Spanned<f64>, low: f64, high: f64, name: &str) -> Result<f32, Problem> {
    let number = *value.get_ref();
    if (low..=high).contains(&number) {
        Ok(number as f32)
    } else {
        Err(problem(
            value.span().start,
            format!("{name} must be between {low} and {high}"),
        ))
    }
}

/// Font style names as Alacritty takes them, to CSS weights.
fn weight_of(style: &str) -> u16 {
    let style = style.to_ascii_lowercase().replace(' ', "");
    let style = style.trim_end_matches("italic");
    match style {
        "thin" | "hairline" => 100,
        "extralight" | "ultralight" => 200,
        "light" => 300,
        "medium" => 500,
        "semibold" | "demibold" => 600,
        "bold" => 700,
        "extrabold" | "ultrabold" => 800,
        "heavy" | "black" => 900,
        _ => 400,
    }
}

fn expand(path: &str, config_dir: &Path, home: Option<&Path>) -> PathBuf {
    if let (Some(rest), Some(home)) = (path.strip_prefix("~/"), home) {
        return home.join(rest);
    }
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else {
        config_dir.join(path)
    }
}

/// Only `[colors]` is read from an imported file (a theme).
pub(crate) fn imported_colors(text: &str) -> Result<ColorSettings, String> {
    #[derive(Deserialize, Default)]
    struct Theme {
        #[serde(default)]
        colors: RawColors,
    }
    let theme: Theme = toml::from_str(text).map_err(|error| error.message().trim().to_owned())?;
    colors(&theme.colors).map_err(|(_, message)| message)
}

pub(crate) fn colors(raw: &RawColors) -> Result<ColorSettings, (usize, String)> {
    let parse = |value: &Option<toml::Spanned<String>>| -> Result<Option<u32>, (usize, String)> {
        let Some(value) = value else {
            return Ok(None);
        };
        let text = value.get_ref().trim();
        let hex = text
            .strip_prefix('#')
            .or_else(|| text.strip_prefix("0x"))
            .unwrap_or(text);
        if hex.len() == 6
            && let Ok(color) = u32::from_str_radix(hex, 16)
        {
            return Ok(Some(color));
        }
        Err((
            value.span().start,
            format!("`{text}` is not a colour; use \"#RRGGBB\""),
        ))
    };
    let eight = |raw: &RawEight| -> Result<[Option<u32>; 8], (usize, String)> {
        Ok([
            parse(&raw.black)?,
            parse(&raw.red)?,
            parse(&raw.green)?,
            parse(&raw.yellow)?,
            parse(&raw.blue)?,
            parse(&raw.magenta)?,
            parse(&raw.cyan)?,
            parse(&raw.white)?,
        ])
    };
    Ok(ColorSettings {
        foreground: parse(&raw.primary.foreground)?,
        background: parse(&raw.primary.background)?,
        cursor: parse(&raw.cursor.cursor)?,
        selection: parse(&raw.selection.background)?,
        normal: eight(&raw.normal)?,
        bright: eight(&raw.bright)?,
        dim: eight(&raw.dim)?,
    })
}

/// Alacritty key names to the app's: "Return" to "enter", "A" to "a", "ArrowUp" to "up".
fn key_name(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    let named = match lower.as_str() {
        "return" | "enter" => "enter",
        "tab" => "tab",
        "back" | "backspace" => "backspace",
        "escape" | "esc" => "escape",
        "space" => "space",
        "up" | "arrowup" => "up",
        "down" | "arrowdown" => "down",
        "left" | "arrowleft" => "left",
        "right" | "arrowright" => "right",
        "home" => "home",
        "end" => "end",
        "pageup" => "pageup",
        "pagedown" => "pagedown",
        "insert" => "insert",
        "delete" => "delete",
        other
            if other.len() >= 2
                && other.starts_with('f')
                && other[1..]
                    .parse::<u8>()
                    .is_ok_and(|n| (1..=12).contains(&n)) =>
        {
            return Some(other.to_owned());
        }
        other if other.chars().count() == 1 => return Some(other.to_owned()),
        _ => return None,
    };
    Some(named.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names_and_weights() {
        assert_eq!(key_name("Return").as_deref(), Some("enter"));
        assert_eq!(key_name("A").as_deref(), Some("a"));
        assert_eq!(key_name("ArrowUp").as_deref(), Some("up"));
        assert_eq!(key_name("F5").as_deref(), Some("f5"));
        assert_eq!(key_name("NumpadEnter"), None);
        assert_eq!(weight_of("Medium"), 500);
        assert_eq!(weight_of("Heavy"), 900);
        assert_eq!(weight_of("Heavy Italic"), 900);
        assert_eq!(weight_of("Regular"), 400);
    }
}
