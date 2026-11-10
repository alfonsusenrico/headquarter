//! The programs the app starts (fix-windows-app D1). On Windows a windowed app that starts a
//! console program gets a console window for it unless it asks for none, so every program the
//! app starts goes through `quiet`.

use std::ffi::OsStr;
use std::process::Command;

/// Windows' process creation flag for a console program that runs without a window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// A command for `program` that opens no window.
pub fn quiet(program: impl AsRef<OsStr>) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut command = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    /// Every `.rs` file under `dir`.
    fn sources(dir: &Path, found: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("the app's sources") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                sources(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                found.push(path);
            }
        }
    }

    #[test]
    fn every_program_the_app_starts_goes_through_quiet() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut files = Vec::new();
        sources(&root, &mut files);
        let needle = ["Command", "::new("].concat();
        let offenders: Vec<String> = files
            .iter()
            .filter(|path| !path.ends_with("process.rs"))
            .filter(|path| {
                std::fs::read_to_string(path)
                    .expect("a source file")
                    .contains(&needle)
            })
            .map(|path| path.display().to_string())
            .collect();
        assert!(
            offenders.is_empty(),
            "start programs with process::quiet: {offenders:?}"
        );
    }
}
