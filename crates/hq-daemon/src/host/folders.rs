//! Folders on the host's computer for a remote desktop's New session folder browser
//! (add-remote-desktop-client D7): names only, read when asked and never stored.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

use hq_discovery::folders;
use hq_protocol::{ErrorCode, Frame, Response};

use crate::session::Outgoing;

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// The folder `text` names: empty for the home folder, `~` or `~/…` under it, or an
/// absolute path. Anything else is refused, so a listing never depends on the host's
/// working folder.
fn resolve(text: &str, home: &Path) -> Option<PathBuf> {
    let text = text.trim();
    if text.is_empty() || text == "~" {
        return Some(home.to_path_buf());
    }
    if let Some(rest) = text.strip_prefix("~/") {
        return Some(home.join(rest));
    }
    let path = Path::new(text);
    path.is_absolute().then(|| path.to_path_buf())
}

/// The answer to `ListFolder`.
pub(super) fn list(text: &str) -> Response {
    let Some(home) = home() else {
        return Response::error(ErrorCode::NotFound, "the host has no home folder");
    };
    let Some(path) = resolve(text, &home) else {
        return Response::error(
            ErrorCode::BadRequest,
            format!("`{text}` is not a folder path; start it with `/` or `~`"),
        );
    };
    match folders::subfolders(&path) {
        Ok(names) => Response::Folder {
            home: home.display().to_string(),
            path: path.display().to_string(),
            folders: names,
        },
        Err(error) => Response::error(
            ErrorCode::FolderNotFound,
            format!("cannot read {}: {}", path.display(), error.kind()),
        ),
    }
}

/// Answers `FindFolders` with batches of the folders under the home folder, on a thread
/// of its own; the walk stops when the client leaves.
pub(super) fn find(tx: mpsc::Sender<Outgoing>) {
    let Some(home) = home() else {
        let _ = tx.send(Outgoing::Frame(Frame::response(&Response::error(
            ErrorCode::NotFound,
            "the host has no home folder",
        ))));
        return;
    };
    thread::spawn(move || {
        let send = |folders: Vec<String>, done: bool| {
            tx.send(Outgoing::Frame(Frame::response(&Response::FoundFolders {
                folders,
                done,
            })))
            .is_ok()
        };
        let mut open = true;
        folders::walk(&home, |batch| {
            let batch = batch
                .into_iter()
                .map(|path| path.display().to_string())
                .collect();
            open = send(batch, false);
            open
        });
        if open {
            send(Vec::new(), true);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_resolve_against_home_only() {
        let home = Path::new("/home/me");
        assert_eq!(resolve("", home), Some(home.to_path_buf()));
        assert_eq!(resolve("~", home), Some(home.to_path_buf()));
        assert_eq!(resolve("~/work", home), Some(home.join("work")));
        assert_eq!(resolve("/srv", home), Some(PathBuf::from("/srv")));
        assert_eq!(resolve("work", home), None, "relative paths are refused");
    }
}
