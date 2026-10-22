//! The optional settings file, `$XDG_CONFIG_HOME/headquarter/config.toml` or
//! `~/.config/headquarter/config.toml`. Unknown keys, wrong types, and out-of-range
//! values are errors naming the file and line, never silently ignored. Alacritty keys
//! Headquarter does not support are warnings, so an Alacritty file can be copied in.

mod appearance;
mod schema;
pub mod setup;
pub mod themes;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::Deserialize;

pub use appearance::{
    AgentMarks, Appearance, ColorSettings, CursorShape, Decorations, FontSettings, KeyBinding,
    OptionAsAlt,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub agents: BTreeMap<String, AgentSettings>,
    /// Whether discovery may run the login shell to learn its `PATH`.
    pub login_shell: bool,
    pub appearance: Appearance,
    pub notifications: Notifications,
    /// Settings that were read but have no effect, such as unsupported Alacritty keys.
    pub warnings: Vec<SettingsError>,
    /// Imported files, watched for changes like the settings file itself.
    pub imports: Vec<PathBuf>,
}

/// Which system notifications the app shows (add-notifications D4); both on by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Notifications {
    /// A sidebar session starts needing the owner.
    pub needs_you: bool,
    /// A sidebar session finishes a turn with a reply not read yet.
    pub replied: bool,
}

impl Default for Notifications {
    fn default() -> Self {
        Notifications {
            needs_you: true,
            replied: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSettings {
    pub enabled: bool,
    pub path: Option<PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            agents: BTreeMap::new(),
            login_shell: true,
            appearance: Appearance::default(),
            notifications: Notifications::default(),
            warnings: Vec::new(),
            imports: Vec::new(),
        }
    }
}

impl Settings {
    pub fn agent_enabled(&self, id: &str) -> bool {
        self.agents.get(id).is_none_or(|agent| agent.enabled)
    }

    pub fn agent_path(&self, id: &str) -> Option<&Path> {
        self.agents.get(id).and_then(|agent| agent.path.as_deref())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsError {
    pub file: PathBuf,
    pub line: Option<usize>,
    pub message: String,
}

impl std::fmt::Display for SettingsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.line {
            Some(line) => write!(f, "{}:{line}: {}", self.file.display(), self.message),
            None => write!(f, "{}: {}", self.file.display(), self.message),
        }
    }
}

impl std::error::Error for SettingsError {}

/// A problem at a byte offset in the source; `parse` turns offsets into lines.
#[derive(Debug)]
pub(crate) struct Problem {
    pub offset: Option<usize>,
    pub message: String,
}

pub(crate) fn problem(offset: usize, message: impl Into<String>) -> Problem {
    Problem {
        offset: Some(offset),
        message: message.into(),
    }
}

// Unknown keys are rejected by `schema::check` before these are read, so these
// structs do not deny unknown fields: unsupported Alacritty keys must pass through.
#[derive(Deserialize, Default)]
struct RawSettings {
    #[serde(default)]
    agents: BTreeMap<String, RawAgent>,
    #[serde(default)]
    discovery: RawDiscovery,
    #[serde(default)]
    notifications: RawNotifications,
    #[serde(default)]
    general: appearance::RawGeneral,
    #[serde(default)]
    window: appearance::RawWindow,
    #[serde(default)]
    font: appearance::RawFont,
    #[serde(default)]
    cursor: appearance::RawCursor,
    #[serde(default)]
    colors: appearance::RawColors,
    #[serde(default)]
    keyboard: appearance::RawKeyboard,
    #[serde(default)]
    sidebar: appearance::RawSidebar,
}

#[derive(Deserialize, Default)]
struct RawAgent {
    enabled: Option<bool>,
    path: Option<toml::Spanned<String>>,
}

#[derive(Deserialize, Default)]
struct RawDiscovery {
    login_shell: Option<bool>,
}

#[derive(Deserialize, Default)]
struct RawNotifications {
    needs_you: Option<bool>,
    replied: Option<bool>,
}

/// Where the settings file lives: `$XDG_CONFIG_HOME/headquarter/config.toml`, or in
/// `HQ_CONFIG_DIR` for development and review runs (see `hq_signals::paths::data_dir`).
pub fn settings_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("HQ_CONFIG_DIR").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(dir).join("config.toml"));
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| home().map(|home| home.join(".config")))?;
    Some(base.join("headquarter").join("config.toml"))
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Reads the settings file; a missing file means defaults. `known_agents` are the
/// agent ids the settings may name.
pub fn load(path: &Path, known_agents: &[&str]) -> Result<Settings, SettingsError> {
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Settings::default());
        }
        Err(error) => {
            return Err(SettingsError {
                file: path.to_owned(),
                line: None,
                message: format!("cannot read: {error}"),
            });
        }
    };
    parse(&source, path, known_agents, home().as_deref())
}

pub fn parse(
    source: &str,
    path: &Path,
    known_agents: &[&str],
    home: Option<&Path>,
) -> Result<Settings, SettingsError> {
    let error_at = |offset: Option<usize>, message: String| SettingsError {
        file: path.to_owned(),
        line: offset.map(|offset| line_of(source, offset)),
        message,
    };
    let from_problem = |problem: Problem| error_at(problem.offset, problem.message);
    let mut warnings = schema::check(source).map_err(from_problem)?;
    let raw: RawSettings = toml::from_str(source).map_err(|error| {
        error_at(
            error.span().map(|span| span.start),
            error.message().trim().to_owned(),
        )
    })?;

    let mut settings = Settings::default();
    let config_dir = path.parent().unwrap_or(Path::new("/"));
    settings.imports = appearance::imports(&raw.general, config_dir, home);
    let parts = appearance::RawAppearance {
        general: raw.general,
        window: raw.window,
        font: raw.font,
        cursor: raw.cursor,
        colors: raw.colors,
        keyboard: raw.keyboard,
        sidebar: raw.sidebar,
    };
    settings.appearance =
        appearance::build(parts, config_dir, home, &mut warnings).map_err(from_problem)?;
    warnings.sort_by_key(|warning| warning.offset);
    settings.warnings = warnings.into_iter().map(from_problem).collect();
    if let Some(login_shell) = raw.discovery.login_shell {
        settings.login_shell = login_shell;
    }
    if let Some(needs_you) = raw.notifications.needs_you {
        settings.notifications.needs_you = needs_you;
    }
    if let Some(replied) = raw.notifications.replied {
        settings.notifications.replied = replied;
    }
    for (id, agent) in raw.agents {
        if !known_agents.contains(&id.as_str()) {
            let offset = source.find(&format!("[agents.{id}]"));
            return Err(error_at(
                offset,
                format!(
                    "unknown agent `{id}`; known agents: {}",
                    known_agents.join(", ")
                ),
            ));
        }
        let path = match agent.path {
            Some(spanned) => Some(expand(spanned.get_ref(), home).ok_or_else(|| {
                error_at(
                    Some(spanned.span().start),
                    "path must be absolute or start with ~/".to_owned(),
                )
            })?),
            None => None,
        };
        settings.agents.insert(
            id,
            AgentSettings {
                enabled: agent.enabled.unwrap_or(true),
                path,
            },
        );
    }
    Ok(settings)
}

fn expand(path: &str, home: Option<&Path>) -> Option<PathBuf> {
    if let Some(rest) = path.strip_prefix("~/") {
        return home.map(|home| home.join(rest));
    }
    let path = PathBuf::from(path);
    path.is_absolute().then_some(path)
}

fn line_of(source: &str, offset: usize) -> usize {
    source[..offset.min(source.len())]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1
}

/// A file's modification time and length, or `None` when it does not exist.
type Stamp = Option<(SystemTime, u64)>;

fn stamp(path: &Path) -> Stamp {
    std::fs::metadata(path)
        .ok()
        .and_then(|metadata| Some((metadata.modified().ok()?, metadata.len())))
}

/// Re-reads the settings file only when it or a file it imports changes, so edits
/// apply without a restart and unchanged files cost one `stat` each.
pub struct SettingsWatcher {
    path: Option<PathBuf>,
    known_agents: Vec<&'static str>,
    /// Stamps of the settings file and its imports, with what they produced.
    cached: Option<(Vec<Stamp>, Result<Settings, SettingsError>)>,
}

impl SettingsWatcher {
    pub fn new(path: Option<PathBuf>, known_agents: Vec<&'static str>) -> SettingsWatcher {
        SettingsWatcher {
            path,
            known_agents,
            cached: None,
        }
    }

    pub fn current(&mut self) -> Result<Settings, SettingsError> {
        let Some(path) = &self.path else {
            return Ok(Settings::default());
        };
        if let Some((seen, result)) = &self.cached
            && *seen == stamps(path, result)
        {
            return result.clone();
        }
        let result = load(path, &self.known_agents);
        self.cached = Some((stamps(path, &result), result.clone()));
        result
    }
}

fn stamps(path: &Path, result: &Result<Settings, SettingsError>) -> Vec<Stamp> {
    let imports = result
        .as_ref()
        .map_or(&[][..], |settings| &settings.imports[..]);
    std::iter::once(path)
        .chain(imports.iter().map(PathBuf::as_path))
        .map(stamp)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const AGENTS: &[&str] = &["claude", "codex", "agy"];

    fn parse_text(source: &str) -> Result<Settings, SettingsError> {
        parse(
            source,
            Path::new("/c/config.toml"),
            AGENTS,
            Some(Path::new("/home/me")),
        )
    }

    #[test]
    fn missing_file_means_defaults() {
        let settings = load(Path::new("/no/such/config.toml"), AGENTS).unwrap();
        assert_eq!(settings, Settings::default());
        assert!(settings.agent_enabled("claude"));
    }

    #[test]
    fn valid_file() {
        let settings = parse_text(
            "[agents.codex]\npath = \"~/bin/codex\"\n\n[agents.agy]\nenabled = false\n\n[discovery]\nlogin_shell = false\n",
        )
        .unwrap();
        assert_eq!(
            settings.agent_path("codex"),
            Some(Path::new("/home/me/bin/codex"))
        );
        assert!(!settings.agent_enabled("agy"));
        assert!(settings.agent_enabled("claude"));
        assert!(!settings.login_shell);
    }

    #[test]
    fn unknown_key_names_its_line() {
        let error = parse_text("[discovery]\nlogin_shell = true\nlogin_shel = 3\n").unwrap_err();
        assert_eq!(error.line, Some(3));
        assert!(error.message.contains("login_shel"), "{error}");
        assert!(error.to_string().starts_with("/c/config.toml:3:"));
    }

    #[test]
    fn wrong_types_are_errors() {
        let wrong_type = parse_text("[discovery]\nlogin_shell = \"yes\"\n").unwrap_err();
        assert_eq!(wrong_type.line, Some(2));
    }

    #[test]
    fn notifications_are_on_unless_turned_off() {
        assert_eq!(
            parse_text("").unwrap().notifications,
            Notifications::default()
        );
        let replied_off = parse_text("[notifications]\nreplied = false\n").unwrap();
        assert_eq!(
            replied_off.notifications,
            Notifications {
                needs_you: true,
                replied: false,
            }
        );
        let wrong = parse_text("[notifications]\nneeds_you = \"no\"\n").unwrap_err();
        assert_eq!(wrong.line, Some(2));
        let unknown = parse_text("[notifications]\nreplies = false\n").unwrap_err();
        assert!(unknown.message.contains("replies"), "{unknown}");
    }

    #[test]
    fn agent_marks_are_mono_unless_another_is_chosen() {
        let default = parse_text("").unwrap();
        assert_eq!(default.appearance.agent_marks, AgentMarks::Mono);
        let color = parse_text("[sidebar]\nagent_marks = \"color\"\n").unwrap();
        assert_eq!(color.appearance.agent_marks, AgentMarks::Color);
        let drawn = parse_text("[sidebar]\nagent_marks = \"drawn\"\n").unwrap();
        assert_eq!(drawn.appearance.agent_marks, AgentMarks::Drawn);
        let wrong = parse_text("[sidebar]\nagent_marks = \"logos\"\n").unwrap_err();
        assert_eq!(wrong.line, Some(2));
        assert!(
            wrong.message.contains("\"mono\", \"color\" or \"drawn\""),
            "{}",
            wrong.message
        );
    }

    #[test]
    fn retired_display_window_is_a_warning() {
        let settings =
            parse_text("[agents.agy]\nenabled = false\n\n[sidebar]\nhistory_days = 7\n").unwrap();
        assert!(
            !settings.agent_enabled("agy"),
            "the rest of the file still applies"
        );
        assert_eq!(settings.warnings.len(), 1);
        assert_eq!(settings.warnings[0].line, Some(5));
        assert!(
            settings.warnings[0]
                .message
                .contains("no longer has an effect"),
            "{}",
            settings.warnings[0]
        );
    }

    #[test]
    fn unknown_agents_and_relative_paths_are_errors() {
        let unknown = parse_text("\n[agents.gpt]\nenabled = true\n").unwrap_err();
        assert_eq!(unknown.line, Some(2));
        assert!(unknown.message.contains("unknown agent `gpt`"));
        let relative = parse_text("[agents.claude]\npath = \"bin/claude\"\n").unwrap_err();
        assert_eq!(relative.line, Some(2));
        assert!(relative.message.contains("absolute"));
    }

    #[test]
    fn watcher_reloads_after_an_edit() {
        let dir = std::env::temp_dir().join(format!("hq-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(&path, "[discovery]\nlogin_shell = false\n").unwrap();
        let mut watcher = SettingsWatcher::new(Some(path.clone()), AGENTS.to_vec());
        assert!(!watcher.current().unwrap().login_shell);
        std::fs::write(&path, "[discovery]\nlogin_shell = oops\n").unwrap();
        assert!(watcher.current().is_err());
        std::fs::write(&path, "[window]\nopacity = 0.5\n").unwrap();
        assert_eq!(watcher.current().unwrap().appearance.opacity, 0.5);
        std::fs::remove_file(&path).unwrap();
        assert!(watcher.current().unwrap().login_shell);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn watcher_reloads_after_an_imported_file_changes() {
        let dir = std::env::temp_dir().join(format!("hq-config-import-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        let theme = dir.join("theme.toml");
        std::fs::write(&theme, "[colors.primary]\nbackground = \"#000000\"\n").unwrap();
        std::fs::write(&path, "[general]\nimport = [\"theme.toml\"]\n").unwrap();
        let mut watcher = SettingsWatcher::new(Some(path.clone()), AGENTS.to_vec());
        let background =
            |watcher: &mut SettingsWatcher| watcher.current().unwrap().appearance.colors.background;
        assert_eq!(background(&mut watcher), Some(0x000000));
        // A different length, so the change shows even where file times are coarse.
        std::fs::write(
            &theme,
            "[colors.primary]\nbackground = \"#1E1E2E\" # base\n",
        )
        .unwrap();
        assert_eq!(background(&mut watcher), Some(0x1E1E2E));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
