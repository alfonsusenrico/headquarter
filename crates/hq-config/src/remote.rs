//! `hq remote on|off` (add-remote-gateway D9): sets `[remote] enabled` in the settings
//! file, keeping its layout and comments, and checks the result still reads.

use std::path::Path;

use toml_edit::DocumentMut;

use crate::SettingsError;

/// Sets `[remote] enabled` in `file`, creating the file when it does not exist. The new
/// text must parse as valid settings before it replaces the old.
pub fn set_enabled(
    file: &Path,
    enabled: bool,
    known_agents: &[&str],
    home: Option<&Path>,
) -> Result<(), SettingsError> {
    let error = |message: String| SettingsError {
        file: file.to_owned(),
        line: None,
        message,
    };
    let existing = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(problem) if problem.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(problem) => return Err(error(format!("cannot read it: {problem}"))),
    };
    let text = with_enabled(&existing, enabled).map_err(error)?;
    crate::parse(&text, file, known_agents, home)?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|problem| error(problem.to_string()))?;
    }
    let partial = file.with_extension(format!("toml.partial{}", std::process::id()));
    std::fs::write(&partial, text).map_err(|problem| error(problem.to_string()))?;
    std::fs::rename(&partial, file).map_err(|problem| error(problem.to_string()))
}

fn with_enabled(existing: &str, enabled: bool) -> Result<String, String> {
    let mut doc: DocumentMut = existing
        .parse()
        .map_err(|problem: toml_edit::TomlError| problem.message().trim().to_owned())?;
    // A plain `[remote]` table, not the inline table indexing would create.
    if !doc.contains_table("remote") {
        doc.insert("remote", toml_edit::Item::Table(toml_edit::Table::new()));
    }
    doc["remote"]["enabled"] = toml_edit::value(enabled);
    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_remote_key_changes() {
        let existing = "# my settings\n[font]\nsize = 14 # big\n\n[remote]\nrelays = [\"https://r.example.com\"]\n";
        let on = with_enabled(existing, true).unwrap();
        assert!(
            on.starts_with("# my settings\n[font]\nsize = 14 # big\n"),
            "{on}"
        );
        assert!(on.contains("enabled = true"), "{on}");
        assert!(on.contains("relays = [\"https://r.example.com\"]"), "{on}");
        let off = with_enabled(&on, false).unwrap();
        assert!(
            off.contains("enabled = false") && !off.contains("enabled = true"),
            "{off}"
        );
    }

    #[test]
    fn a_missing_file_gets_just_the_section() {
        assert_eq!(
            with_enabled("", true).unwrap().trim(),
            "[remote]\nenabled = true"
        );
    }

    #[test]
    fn writes_and_refuses_broken_files() {
        let dir = std::env::temp_dir().join(format!("hq-remote-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("config.toml");
        set_enabled(&file, true, &["claude"], None).unwrap();
        assert!(
            std::fs::read_to_string(&file)
                .unwrap()
                .contains("enabled = true")
        );
        std::fs::write(&file, "[font\n").unwrap();
        assert!(set_enabled(&file, false, &["claude"], None).is_err());
        assert_eq!(
            std::fs::read_to_string(&file).unwrap(),
            "[font\n",
            "left alone"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
