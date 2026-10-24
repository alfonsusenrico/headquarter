//! Which keys the settings file may contain. Headquarter's own keys and the Alacritty
//! keys it supports are accepted; Alacritty keys it does not support are reported as
//! warnings so a copied Alacritty file still loads; anything else is an error.

use toml::Spanned;
use toml::de::{DeTable, DeValue};

use crate::{Problem, problem};

/// Key paths that are read. `*` stands for any one key.
const SUPPORTED: &[&str] = &[
    "agents.*.enabled",
    "agents.*.path",
    "discovery.login_shell",
    "notifications.needs_you",
    "notifications.replied",
    "updates.check",
    "general.import",
    "general.live_config_reload",
    "window.opacity",
    "window.blur",
    "window.padding.x",
    "window.padding.y",
    "window.decorations",
    "window.option_as_alt",
    "window.dim_unfocused",
    "sidebar.agent_marks",
    "font.size",
    "font.*.family",
    "font.*.style",
    "cursor.style",
    "cursor.style.shape",
    "colors.primary.foreground",
    "colors.primary.background",
    "colors.cursor.cursor",
    "colors.selection.background",
    "colors.selection.text",
    "colors.normal.*",
    "colors.bright.*",
    "colors.dim.*",
    "keyboard.bindings.key",
    "keyboard.bindings.mods",
    "keyboard.bindings.chars",
];

/// Headquarter keys that once had an effect, with what the owner should know. They
/// still load, so an older settings file keeps working.
const RETIRED: &[(&str, &str)] = &[(
    "sidebar.history_days",
    "no longer has an effect: the sidebar shows the sessions you add, and `hq ls --all` lists every session",
)];

/// Alacritty keys Headquarter does not offer. A trailing `**` covers a whole subtree.
const UNSUPPORTED: &[&str] = &[
    "general.working_directory",
    "general.ipc_socket",
    "env.**",
    "window.dimensions.**",
    "window.position.**",
    "window.dynamic_padding",
    "window.startup_mode",
    "window.title",
    "window.dynamic_title",
    "window.class.**",
    "window.decorations_theme_variant",
    "window.resize_increments",
    "window.level",
    "scrolling.**",
    "font.offset.**",
    "font.glyph_offset.**",
    "font.builtin_box_drawing",
    "colors.primary.dim_foreground",
    "colors.primary.bright_foreground",
    "colors.cursor.text",
    "colors.vi_mode_cursor.**",
    "colors.search.**",
    "colors.hints.**",
    "colors.line_indicator.**",
    "colors.footer_bar.**",
    "colors.indexed_colors",
    "colors.transparent_background_colors",
    "colors.draw_bold_text_with_bright_colors",
    "bell.**",
    "selection.**",
    "cursor.style.blinking",
    "cursor.vi_mode_style",
    "cursor.blink_interval",
    "cursor.blink_timeout",
    "cursor.unfocused_hollow",
    "cursor.thickness",
    "terminal.**",
    "mouse.**",
    "hints.**",
    "keyboard.bindings.mode",
    "keyboard.bindings.action",
    "keyboard.bindings.command",
    "debug.**",
];

/// Checks every key in `source`. Returns the warnings, or the first error.
pub(crate) fn check(source: &str) -> Result<Vec<Problem>, Problem> {
    let table = DeTable::parse(source).map_err(|error| Problem {
        offset: error.span().map(|span| span.start),
        message: error.message().trim().to_owned(),
    })?;
    let mut warnings = Vec::new();
    walk(table.get_ref(), &mut Vec::new(), &mut warnings)?;
    warnings.sort_by_key(|warning| warning.offset);
    Ok(warnings)
}

fn walk<'a>(
    table: &'a DeTable<'_>,
    path: &mut Vec<&'a str>,
    warnings: &mut Vec<Problem>,
) -> Result<(), Problem> {
    for (key, value) in table {
        path.push(key.get_ref());
        visit(key, value.get_ref(), path, warnings)?;
        path.pop();
    }
    Ok(())
}

fn visit<'a>(
    key: &Spanned<std::borrow::Cow<'_, str>>,
    value: &'a DeValue<'_>,
    path: &mut Vec<&'a str>,
    warnings: &mut Vec<Problem>,
) -> Result<(), Problem> {
    let offset = key.span().start;
    let name = path.join(".");
    if let Some((_, note)) = RETIRED.iter().find(|(pattern, _)| matches(pattern, path)) {
        warnings.push(problem(offset, format!("`{name}` {note}")));
        return Ok(());
    }
    if any_match(UNSUPPORTED, path) {
        warnings.push(problem(
            offset,
            format!(
                "`{name}` is an Alacritty setting Headquarter does not support; it has no effect"
            ),
        ));
        return Ok(());
    }
    let tables: Vec<&DeTable<'_>> = match value {
        DeValue::Table(table) => vec![table],
        DeValue::Array(items) if !items.is_empty() => items
            .iter()
            .filter_map(|item| match item.get_ref() {
                DeValue::Table(table) => Some(table),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    if !tables.is_empty() && any_prefix(path) {
        for table in tables {
            walk(table, path, warnings)?;
        }
        return Ok(());
    }
    if any_match(SUPPORTED, path) {
        return Ok(());
    }
    Err(problem(offset, format!("unknown setting `{name}`")))
}

fn segments(pattern: &str) -> Vec<&str> {
    pattern.split('.').collect()
}

fn matches(pattern: &str, path: &[&str]) -> bool {
    let parts = segments(pattern);
    if parts.last() == Some(&"**") {
        let fixed = &parts[..parts.len() - 1];
        return path.len() >= fixed.len() && same(fixed, &path[..fixed.len()]);
    }
    parts.len() == path.len() && same(&parts, path)
}

fn same(pattern: &[&str], path: &[&str]) -> bool {
    pattern
        .iter()
        .zip(path)
        .all(|(pattern, key)| *pattern == "*" || pattern == key)
}

fn any_match(patterns: &[&str], path: &[&str]) -> bool {
    patterns.iter().any(|pattern| matches(pattern, path))
}

/// Whether some known key lies below `path`, so a table at `path` is worth entering.
fn any_prefix(path: &[&str]) -> bool {
    let retired = RETIRED.iter().map(|(pattern, _)| pattern);
    SUPPORTED
        .iter()
        .chain(UNSUPPORTED)
        .chain(retired)
        .any(|pattern| {
            let parts = segments(pattern);
            parts.len() > path.len() && same(&parts[..path.len()], path)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages(source: &str) -> Result<Vec<String>, String> {
        check(source)
            .map(|warnings| {
                warnings
                    .into_iter()
                    .map(|warning| warning.message)
                    .collect()
            })
            .map_err(|error| error.message)
    }

    #[test]
    fn supported_keys_pass() {
        let source = "[window]\nopacity = 0.7\npadding = { x = 10, y = 10 }\n\n[font.normal]\nfamily = \"X\"\n\n[cursor.style]\nshape = \"Beam\"\n\n[[keyboard.bindings]]\nkey = \"Return\"\nmods = \"Shift\"\nchars = \"\\u001B\\r\"\n\n[agents.codex]\nenabled = false\n";
        assert_eq!(messages(source), Ok(Vec::new()));
    }

    #[test]
    fn unsupported_alacritty_keys_warn_with_their_line() {
        let source = "[env]\nTERM = \"xterm-256color\"\n\n[cursor]\nstyle = { shape = \"Beam\", blinking = \"On\" }\n";
        let warnings = check(source).unwrap();
        assert_eq!(warnings.len(), 2);
        assert!(warnings[0].message.contains("`env`"));
        assert!(warnings[1].message.contains("`cursor.style.blinking`"));
        let line = |offset: usize| source[..offset].lines().count();
        assert_eq!(line(warnings[0].offset.unwrap()), 1);
        assert_eq!(line(warnings[1].offset.unwrap()), 5);
    }

    #[test]
    fn unknown_keys_are_errors() {
        assert_eq!(
            messages("[sidebar]\nhistroy_dayz = 3\n"),
            Err("unknown setting `sidebar.histroy_dayz`".to_owned())
        );
        assert_eq!(
            messages("[window]\nopacityy = 1.0\n"),
            Err("unknown setting `window.opacityy`".to_owned())
        );
        assert_eq!(
            messages("[nonsense]\n"),
            Err("unknown setting `nonsense`".to_owned())
        );
        assert_eq!(
            messages("[agents.codex]\nflags = 1\n"),
            Err("unknown setting `agents.codex.flags`".to_owned())
        );
    }

    #[test]
    fn syntax_errors_keep_their_position() {
        let error = check("[window]\nopacity = \n").unwrap_err();
        assert!(error.offset.is_some());
    }
}
