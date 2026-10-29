//! Whether Headquarter's terminal is light or dark, kept in the data folder by the app so the
//! host can tell agents at launch, including sessions it starts with no window open
//! (fix-light-theme-contrast D2).

use std::path::Path;

use crate::paths;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Background {
    Light,
    Dark,
}

impl Background {
    fn word(self) -> &'static str {
        match self {
            Background::Light => "light",
            Background::Dark => "dark",
        }
    }
}

/// The last background the app wrote, if any.
pub fn read() -> Option<Background> {
    read_from(&paths::data_dir()?.join(FILE))
}

/// Keeps `background` for the host; written whole, so a reader never sees half a file.
pub fn write(background: Background) -> std::io::Result<()> {
    let dir = paths::data_dir().ok_or_else(|| std::io::Error::other("no data folder"))?;
    write_to(&dir.join(FILE), background)
}

const FILE: &str = "terminal-background";

fn read_from(path: &Path) -> Option<Background> {
    match std::fs::read_to_string(path).ok()?.trim() {
        "light" => Some(Background::Light),
        "dark" => Some(Background::Dark),
        _ => None,
    }
}

fn write_to(path: &Path, background: Background) -> std::io::Result<()> {
    if read_from(path) == Some(background) {
        return Ok(());
    }
    let partial = path.with_extension("partial");
    std::fs::write(&partial, background.word())?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_background_is_kept_whole() {
        let dir = std::env::temp_dir().join(format!("hq-background-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE);
        assert_eq!(read_from(&path), None, "nothing written yet");
        write_to(&path, Background::Light).unwrap();
        assert_eq!(read_from(&path), Some(Background::Light));
        write_to(&path, Background::Dark).unwrap();
        assert_eq!(read_from(&path), Some(Background::Dark));
        std::fs::write(&path, "sepia").unwrap();
        assert_eq!(read_from(&path), None, "unknown words are ignored");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
