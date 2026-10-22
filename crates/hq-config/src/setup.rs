//! Writing the settings file from first-run setup, and editing it in place when setup
//! runs again (`add-first-run-setup` design D3, D4). Only setup's keys are ever
//! touched: each agent's `enabled`, and the look keys below.

use std::path::{Path, PathBuf};

use toml_edit::{DocumentMut, Item, Table};

use crate::appearance::{self, Appearance, ColorSettings, CursorShape, Decorations, OptionAsAlt};
use crate::{RawSettings, SettingsError};

/// The settings setup's "look" step owns, as paths into the file.
const LOOK_KEYS: &[&[&str]] = &[
    &["general", "import"],
    &["window", "opacity"],
    &["window", "blur"],
    &["window", "padding"],
    &["window", "decorations"],
    &["window", "option_as_alt"],
    &["font", "size"],
    &["font", "normal"],
    &["font", "bold"],
    &["font", "italic"],
    &["font", "bold_italic"],
    &["cursor", "style"],
    &["colors", "primary"],
    &["colors", "cursor"],
    &["colors", "selection"],
    &["colors", "normal"],
    &["colors", "bright"],
    &["colors", "dim"],
    &["keyboard", "bindings"],
];

/// The owner's Alacritty look, as setup offers it.
#[derive(Debug, Clone, PartialEq)]
pub struct AlacrittyLook {
    /// The look as it applies, with imported colours merged.
    pub appearance: Appearance,
    /// Colours set in the Alacritty file itself, without the imported ones.
    pub own_colors: ColorSettings,
    /// Imported theme files, as absolute paths, so they keep being read from there.
    pub imports: Vec<PathBuf>,
    /// Alacritty settings that do not carry over, each with its line.
    pub not_carried: Vec<SettingsError>,
}

/// Where Alacritty reads its settings, first match wins, as Alacritty itself looks.
pub fn alacritty_path(config_home: Option<&Path>, home: Option<&Path>) -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(config_home) = config_home {
        candidates.push(config_home.join("alacritty/alacritty.toml"));
    }
    if let Some(home) = home {
        candidates.push(home.join(".config/alacritty/alacritty.toml"));
        candidates.push(home.join(".alacritty.toml"));
    }
    candidates.into_iter().find(|path| path.is_file())
}

/// Reads an Alacritty settings file with Headquarter's own rules.
pub fn read_alacritty(path: &Path, home: Option<&Path>) -> Result<AlacrittyLook, SettingsError> {
    let source = std::fs::read_to_string(path).map_err(|error| SettingsError {
        file: path.to_owned(),
        line: None,
        message: format!("cannot read: {error}"),
    })?;
    let settings = crate::parse(&source, path, &[], home)?;
    let raw: RawSettings = toml::from_str(&source).map_err(|error| SettingsError {
        file: path.to_owned(),
        line: None,
        message: error.message().trim().to_owned(),
    })?;
    let own_colors = appearance::colors(&raw.colors).map_err(|(_, message)| SettingsError {
        file: path.to_owned(),
        line: None,
        message,
    })?;
    Ok(AlacrittyLook {
        appearance: settings.appearance,
        own_colors,
        imports: settings.imports,
        not_carried: settings.warnings,
    })
}

/// What the owner chose in setup.
#[derive(Debug, Clone, Default)]
pub struct Choices<'a> {
    /// Agent ids to turn off.
    pub agents_off: Vec<String>,
    /// Agent ids to turn on (an `enabled = false` is removed).
    pub agents_on: Vec<String>,
    /// `None` is Headquarter's own look.
    pub look: Option<&'a AlacrittyLook>,
}

/// A new settings file.
pub fn render(choices: &Choices<'_>, date: &str) -> String {
    let mut doc = DocumentMut::new();
    apply_agents(&mut doc, choices);
    if let Some(look) = choices.look {
        set_look(&mut doc, look);
        if !look.appearance.live_reload {
            doc["general"]["live_config_reload"] = toml_edit::value(false);
        }
    }
    let body = doc.to_string();
    let header = format!(
        "# Headquarter settings, written by setup on {date}. Every key is optional and\n\
         # edits apply while the app runs; see the README.\n"
    );
    if body.trim().is_empty() {
        header
    } else {
        format!("{header}\n{}", body.trim_start())
    }
}

/// An edited settings file and what the owner should be told about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edited {
    pub text: String,
    /// Look settings kept because they differ from what the Alacritty import wrote.
    pub kept: Vec<String>,
}

/// Changes only setup's keys in `existing`. Choosing Headquarter's own look removes a
/// look setting only when it still equals what `previous_look` would write; others are
/// kept and named. The result must read back without errors.
pub fn edit(
    existing: &str,
    file: &Path,
    choices: &Choices<'_>,
    previous_look: Option<&AlacrittyLook>,
    known_agents: &[&str],
) -> Result<Edited, SettingsError> {
    let mut doc: DocumentMut =
        existing
            .parse()
            .map_err(|error: toml_edit::TomlError| SettingsError {
                file: file.to_owned(),
                line: None,
                message: error.message().trim().to_owned(),
            })?;
    apply_agents(&mut doc, choices);
    let mut kept = Vec::new();
    match choices.look {
        Some(look) => {
            for path in LOOK_KEYS {
                remove(&mut doc, path);
            }
            set_look(&mut doc, look);
        }
        None => {
            let imported = previous_look.map(look_document);
            for path in LOOK_KEYS {
                let Some(current) = get(&doc, path).and_then(canonical) else {
                    continue;
                };
                let same = imported
                    .as_ref()
                    .and_then(|imported| get(imported, path))
                    .and_then(canonical)
                    .is_some_and(|written| written == current);
                if same {
                    remove(&mut doc, path);
                } else {
                    kept.push(path.join("."));
                }
            }
        }
    }
    prune(&mut doc);
    let text = doc.to_string();
    crate::parse(&text, file, known_agents, None)?;
    Ok(Edited { text, kept })
}

fn apply_agents(doc: &mut DocumentMut, choices: &Choices<'_>) {
    for id in &choices.agents_off {
        if let Some(table) = table_at(doc, &["agents", id.as_str()]) {
            table.insert("enabled", toml_edit::value(false));
        }
    }
    for id in &choices.agents_on {
        remove(doc, &["agents", id.as_str(), "enabled"]);
    }
}

/// The position after every table in the file, so new tables go at the end.
fn next_position(doc: &DocumentMut) -> isize {
    fn highest(table: &Table) -> isize {
        let mut top = table.position().unwrap_or(0);
        for (_, item) in table.iter() {
            if let Some(inner) = item.as_table() {
                top = top.max(highest(inner));
            }
            if let Some(array) = item.as_array_of_tables() {
                for inner in array.iter() {
                    top = top.max(highest(inner));
                }
            }
        }
        top
    }
    highest(doc.as_table()) + 1
}

/// The table at `path`, created at the end of the file when missing. Created tables
/// are implicit: a header appears only once they hold values, so no empty `[agents]`
/// or `[keyboard]` is written.
fn table_at<'a>(
    doc: &'a mut DocumentMut,
    path: &[&str],
) -> Option<&'a mut dyn toml_edit::TableLike> {
    let mut position = next_position(doc);
    let mut table = doc.as_table_mut() as &mut dyn toml_edit::TableLike;
    for key in path {
        if table.get(key).and_then(Item::as_table_like).is_none() {
            let mut fresh = Table::new();
            fresh.set_implicit(true);
            fresh.set_position(Some(position));
            position += 1;
            table.insert(key, Item::Table(fresh));
        }
        table = table.get_mut(key)?.as_table_like_mut()?;
    }
    Some(table)
}

/// Numbers a copied item's tables from `position`, so they go at the end of the file
/// in order rather than where they sat in the document they came from.
fn place(item: &mut Item, position: &mut isize) {
    fn place_table(table: &mut Table, position: &mut isize) {
        table.set_position(Some(*position));
        *position += 1;
        for (_, inner) in table.iter_mut() {
            place(inner, position);
        }
    }
    match item {
        Item::Table(table) => place_table(table, position),
        Item::ArrayOfTables(array) => {
            for table in array.iter_mut() {
                place_table(table, position);
            }
        }
        _ => {}
    }
}

fn get<'a>(doc: &'a DocumentMut, path: &[&str]) -> Option<&'a Item> {
    let (last, parents) = path.split_last()?;
    let mut table = doc.as_table() as &dyn toml_edit::TableLike;
    for key in parents {
        table = table.get(key)?.as_table_like()?;
    }
    table.get(last)
}

fn remove(doc: &mut DocumentMut, path: &[&str]) {
    let Some((last, parents)) = path.split_last() else {
        return;
    };
    let mut table = doc.as_table_mut() as &mut dyn toml_edit::TableLike;
    for key in parents {
        let Some(next) = table.get_mut(key).and_then(Item::as_table_like_mut) else {
            return;
        };
        table = next;
    }
    table.remove(last);
}

/// Removes tables left empty by removals, innermost first.
fn prune(doc: &mut DocumentMut) {
    fn prune_table(table: &mut dyn toml_edit::TableLike) {
        let keys: Vec<String> = table.iter().map(|(key, _)| key.to_owned()).collect();
        for key in keys {
            let empty = match table.get_mut(&key) {
                Some(item) => match item.as_table_like_mut() {
                    Some(inner) => {
                        prune_table(inner);
                        inner.is_empty()
                    }
                    None => false,
                },
                None => false,
            };
            if empty {
                table.remove(&key);
            }
        }
    }
    prune_table(doc.as_table_mut());
}

/// A value compared by meaning, not layout: `padding = { x = 10 }` equals
/// `padding.x = 10`.
fn canonical(item: &Item) -> Option<toml::Value> {
    let mut doc = DocumentMut::new();
    let mut item = item.clone();
    if let Some(table) = item.as_table_mut() {
        table.set_implicit(false);
    }
    doc.insert("v", item);
    let parsed: toml::Table = toml::from_str(&doc.to_string()).ok()?;
    parsed.get("v").cloned()
}

fn set_look(doc: &mut DocumentMut, look: &AlacrittyLook) {
    let written = look_document(look);
    for path in LOOK_KEYS {
        let Some(item) = get(&written, path) else {
            continue;
        };
        let Some((last, parents)) = path.split_last() else {
            continue;
        };
        let mut item = item.clone();
        let mut position = next_position(doc);
        place(&mut item, &mut position);
        if let Some(table) = table_at(doc, parents) {
            table.insert(last, item);
        }
    }
}

/// The look as setup writes it, in Alacritty's names; only settings that differ from
/// the defaults are written.
fn look_document(look: &AlacrittyLook) -> DocumentMut {
    let text = look_text(look);
    text.parse().unwrap_or_default()
}

fn quoted(text: &str) -> String {
    toml::Value::String(text.to_owned()).to_string()
}

fn look_text(look: &AlacrittyLook) -> String {
    let defaults = Appearance::default();
    let a = &look.appearance;
    let mut out = String::new();

    if !look.imports.is_empty() {
        let imports: Vec<String> = look
            .imports
            .iter()
            .map(|path| quoted(&path.to_string_lossy()))
            .collect();
        out.push_str(&format!("[general]\nimport = [{}]\n\n", imports.join(", ")));
    }

    let mut window = String::new();
    if a.opacity != defaults.opacity {
        window.push_str(&format!("opacity = {}\n", a.opacity));
    }
    if a.blur {
        window.push_str("blur = true\n");
    }
    if a.padding != defaults.padding {
        window.push_str(&format!(
            "padding = {{ x = {}, y = {} }}\n",
            a.padding.0, a.padding.1
        ));
    }
    if a.decorations != defaults.decorations {
        let name = match a.decorations {
            Decorations::Full => "Full",
            Decorations::Buttonless => "Buttonless",
            Decorations::None => "None",
        };
        window.push_str(&format!("decorations = \"{name}\"\n"));
    }
    if a.option_as_alt != defaults.option_as_alt {
        let name = match a.option_as_alt {
            OptionAsAlt::None => "None",
            OptionAsAlt::Left => "OnlyLeft",
            OptionAsAlt::Right => "OnlyRight",
            OptionAsAlt::Both => "Both",
        };
        window.push_str(&format!("option_as_alt = \"{name}\"\n"));
    }
    if !window.is_empty() {
        out.push_str(&format!("[window]\n{window}\n"));
    }

    let font = &a.font;
    let mut fonts = String::new();
    if let Some(size) = font.size {
        fonts.push_str(&format!("size = {size}\n"));
    }
    let face = |family: Option<&str>, weight: Option<u16>, italic: bool| {
        let mut parts = Vec::new();
        if let Some(family) = family {
            parts.push(format!("family = {}", quoted(family)));
        }
        if let Some(weight) = weight {
            parts.push(format!("style = {}", quoted(&style_name(weight, italic))));
        }
        (!parts.is_empty()).then(|| format!("{{ {} }}", parts.join(", ")))
    };
    let faces = [
        (
            "normal",
            face(font.family.as_deref(), font.normal_weight, false),
        ),
        ("bold", face(None, font.bold_weight, false)),
        ("italic", face(None, font.italic_weight, true)),
        ("bold_italic", face(None, font.bold_italic_weight, true)),
    ];
    for (name, value) in faces {
        if let Some(value) = value {
            fonts.push_str(&format!("{name} = {value}\n"));
        }
    }
    if !fonts.is_empty() {
        out.push_str(&format!("[font]\n{fonts}\n"));
    }

    if let Some(cursor) = a.cursor {
        let name = match cursor {
            CursorShape::Block => "Block",
            CursorShape::Beam => "Beam",
            CursorShape::Underline => "Underline",
        };
        out.push_str(&format!("[cursor]\nstyle = \"{name}\"\n\n"));
    }

    out.push_str(&colors_text(&look.own_colors));

    for binding in &a.bindings {
        let mut mods = Vec::new();
        for (on, name) in [
            (binding.shift, "Shift"),
            (binding.control, "Control"),
            (binding.alt, "Alt"),
            (binding.platform, "Super"),
        ] {
            if on {
                mods.push(name);
            }
        }
        out.push_str("[[keyboard.bindings]]\n");
        out.push_str(&format!("key = {}\n", quoted(&binding.key)));
        if !mods.is_empty() {
            out.push_str(&format!("mods = \"{}\"\n", mods.join("|")));
        }
        out.push_str(&format!(
            "chars = {}\n\n",
            quoted(&String::from_utf8_lossy(&binding.chars))
        ));
    }
    out
}

fn style_name(weight: u16, italic: bool) -> String {
    let name = match weight {
        ..=150 => "Thin",
        151..=250 => "ExtraLight",
        251..=350 => "Light",
        351..=450 => "Regular",
        451..=550 => "Medium",
        551..=650 => "SemiBold",
        651..=750 => "Bold",
        751..=850 => "ExtraBold",
        _ => "Heavy",
    };
    match (italic, name) {
        (true, "Regular") => "Italic".to_owned(),
        (true, name) => format!("{name} Italic"),
        (false, name) => name.to_owned(),
    }
}

fn colors_text(colors: &ColorSettings) -> String {
    let hex = |color: u32| format!("\"#{color:06X}\"");
    let mut out = String::new();
    let mut section = |name: &str, entries: Vec<(&str, Option<u32>)>| {
        let lines: Vec<String> = entries
            .into_iter()
            .filter_map(|(key, color)| color.map(|color| format!("{key} = {}", hex(color))))
            .collect();
        if !lines.is_empty() {
            out.push_str(&format!("[colors.{name}]\n{}\n\n", lines.join("\n")));
        }
    };
    section(
        "primary",
        vec![
            ("foreground", colors.foreground),
            ("background", colors.background),
        ],
    );
    section("cursor", vec![("cursor", colors.cursor)]);
    section("selection", vec![("background", colors.selection)]);
    const NAMES: [&str; 8] = [
        "black", "red", "green", "yellow", "blue", "magenta", "cyan", "white",
    ];
    for (name, set) in [
        ("normal", &colors.normal),
        ("bright", &colors.bright),
        ("dim", &colors.dim),
    ] {
        section(
            name,
            NAMES.iter().copied().zip(set.iter().copied()).collect(),
        );
    }
    out
}

/// Where a rerun keeps the previous file: beside it, named by the time.
pub fn backup_path(file: &Path, stamp: &str) -> PathBuf {
    let name = file.file_name().map_or_else(
        || "config.toml".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    file.with_file_name(format!("{name}.before-setup-{stamp}"))
}

/// First run: writes `text` only if no file exists, through a temporary file renamed
/// into place, so a partly written file is never left behind. `Ok(false)` when a file
/// appeared meanwhile and was left alone.
pub fn write_new(file: &Path, text: &str) -> std::io::Result<bool> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let partial = file.with_extension(format!("toml.partial{}", std::process::id()));
    std::fs::write(&partial, text)?;
    // `hard_link` fails when the target exists, unlike `rename`, so a file created in
    // the meantime is never replaced.
    let linked = std::fs::hard_link(&partial, file);
    let _ = std::fs::remove_file(&partial);
    match linked {
        Ok(()) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(false),
        Err(error) => Err(error),
    }
}

/// Rerun: copies the current file to `backup`, then replaces it with `text` through a
/// temporary file.
pub fn replace_with_copy(file: &Path, backup: &Path, text: &str) -> std::io::Result<()> {
    std::fs::copy(file, backup)?;
    let partial = file.with_extension(format!("toml.partial{}", std::process::id()));
    std::fs::write(&partial, text)?;
    std::fs::rename(&partial, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGENTS: &[&str] = &["claude", "codex", "agy"];

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
    }

    fn owner_look() -> AlacrittyLook {
        let home = fixtures().join("home");
        read_alacritty(&fixtures().join("alacritty.toml"), Some(&home)).unwrap()
    }

    fn read_back(text: &str) -> crate::Settings {
        crate::parse(text, Path::new("/c/config.toml"), AGENTS, None).unwrap()
    }

    #[test]
    fn owner_alacritty_file_is_read_with_what_does_not_carry_over() {
        let look = owner_look();
        assert!((look.appearance.opacity - 0.7).abs() < f32::EPSILON);
        assert!(
            look.own_colors.is_empty(),
            "the fixture's colours are all imported"
        );
        assert_eq!(look.imports.len(), 1);
        assert!(look.imports[0].is_absolute());
        assert_eq!(look.not_carried.len(), 1);
        assert!(look.not_carried[0].message.contains("`env`"));
    }

    #[test]
    fn broken_alacritty_file_names_its_line() {
        let dir = std::env::temp_dir().join(format!("hq-setup-broken-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("alacritty.toml");
        std::fs::write(&path, "[window]\nopacity = \"high\"\n").unwrap();
        let error = read_alacritty(&path, None).unwrap_err();
        assert_eq!(error.line, Some(2));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rendered_files_read_back_the_same() {
        let defaults = render(&Choices::default(), "2026-10-20");
        assert!(defaults.starts_with("# Headquarter settings"));
        let settings = read_back(&defaults);
        assert_eq!(settings, crate::Settings::default());

        let off = render(
            &Choices {
                agents_off: vec!["agy".into()],
                ..Choices::default()
            },
            "2026-10-20",
        );
        let settings = read_back(&off);
        assert!(!settings.agent_enabled("agy"));
        assert!(settings.agent_enabled("codex"));
        assert!(settings.warnings.is_empty());

        let look = owner_look();
        let alacritty = render(
            &Choices {
                agents_off: vec!["agy".into()],
                look: Some(&look),
                ..Choices::default()
            },
            "2026-10-20",
        );
        let settings = read_back(&alacritty);
        assert!(settings.warnings.is_empty(), "{:?}", settings.warnings);
        assert_eq!(settings.appearance, look.appearance, "{alacritty}");
        assert_eq!(settings.imports, look.imports);
        assert!(!alacritty.contains("[env]"));
    }

    const HAND_WRITTEN: &str = "# My Headquarter settings\n\n[agents.codex]\npath = \"/opt/codex\" # pinned\n\n[discovery]\nlogin_shell = false\n\n[window]\ndim_unfocused = 0.2\nopacity = 0.9\n";

    #[test]
    fn rerun_changes_only_setups_keys() {
        let edited = edit(
            HAND_WRITTEN,
            Path::new("/c/config.toml"),
            &Choices {
                agents_off: vec!["agy".into()],
                ..Choices::default()
            },
            None,
            AGENTS,
        )
        .unwrap();
        assert!(
            edited.text.starts_with(HAND_WRITTEN.trim_end()),
            "{}",
            edited.text
        );
        assert!(
            edited.text.contains("[agents.agy]\nenabled = false"),
            "{}",
            edited.text
        );
        assert_eq!(
            edited.kept,
            ["window.opacity"],
            "a hand-set look key is kept and named"
        );

        let back_on = edit(
            &edited.text,
            Path::new("/c/config.toml"),
            &Choices {
                agents_on: vec!["agy".into()],
                ..Choices::default()
            },
            None,
            AGENTS,
        )
        .unwrap();
        assert!(!back_on.text.contains("agy"), "{}", back_on.text);
        assert!(back_on.text.contains("path = \"/opt/codex\" # pinned"));
    }

    #[test]
    fn switching_back_removes_only_unchanged_imported_settings() {
        let look = owner_look();
        let imported = edit(
            HAND_WRITTEN,
            Path::new("/c/config.toml"),
            &Choices {
                look: Some(&look),
                ..Choices::default()
            },
            None,
            AGENTS,
        )
        .unwrap();
        assert!(
            imported
                .text
                .starts_with("# My Headquarter settings\n\n[agents.codex]"),
            "{}",
            imported.text
        );
        assert!(!imported.text.contains("[keyboard]\n"), "{}", imported.text);
        let settings = read_back(&imported.text);
        assert!((settings.appearance.opacity - 0.7).abs() < f32::EPSILON);
        assert!((settings.appearance.dim_unfocused - 0.2).abs() < f32::EPSILON);

        // The owner changes the opacity by hand, using dotted keys elsewhere.
        let hand_changed = imported
            .text
            .replace("opacity = 0.7", "opacity = 0.85")
            .replace(
                "padding = { x = 10, y = 10 }",
                "padding.x = 10\npadding.y = 10",
            );
        let back = edit(
            &hand_changed,
            Path::new("/c/config.toml"),
            &Choices::default(),
            Some(&look),
            AGENTS,
        )
        .unwrap();
        assert_eq!(back.kept, ["window.opacity"]);
        let settings = read_back(&back.text);
        assert!((settings.appearance.opacity - 0.85).abs() < f32::EPSILON);
        assert_eq!(
            settings.appearance.padding,
            (12, 10),
            "padding matched and went"
        );
        assert!(settings.appearance.font.family.is_none());
        assert!(settings.appearance.bindings.is_empty());
        assert!(back.text.contains("dim_unfocused = 0.2"));
        assert!(back.text.contains("# My Headquarter settings"));
        assert!(back.text.contains("login_shell = false"));
    }

    #[test]
    fn an_edit_that_would_not_read_back_writes_nothing() {
        let error = edit(
            "[agents.gpt]\nenabled = true\n",
            Path::new("/c/config.toml"),
            &Choices::default(),
            None,
            AGENTS,
        )
        .unwrap_err();
        assert!(error.message.contains("unknown agent"), "{error}");
    }

    #[test]
    fn writing_new_never_replaces_and_reruns_keep_a_copy() {
        let dir = std::env::temp_dir().join(format!("hq-setup-write-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("headquarter/config.toml");
        assert!(write_new(&file, "# first\n").unwrap());
        assert!(
            !write_new(&file, "# second\n").unwrap(),
            "a file that exists stays"
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "# first\n");

        let backup = backup_path(&file, "20261020-101500");
        assert!(backup.ends_with("config.toml.before-setup-20261020-101500"));
        replace_with_copy(&file, &backup, "# edited\n").unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "# edited\n");
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), "# first\n");
        let leftovers: Vec<_> = std::fs::read_dir(file.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains("partial"))
            .collect();
        assert!(leftovers.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
