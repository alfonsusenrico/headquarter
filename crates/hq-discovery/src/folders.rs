//! Folders on this computer for the New session folder browser
//! (redesign-session-modal D6): names only, read when asked and never stored. The app
//! reads them for its own computer's host, and the host answers remote clients with them
//! (add-remote-desktop-client D7).

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};

/// At most this many subfolders are listed for one folder.
pub const MAX_LISTED: usize = 200;
/// The walk that finds folders by name stops this many levels below the home folder…
pub const WALK_DEPTH: usize = 8;
/// …or after this many folders.
pub const WALK_LIMIT: usize = 50_000;
/// At most this many folders found by name are listed.
pub const MAX_FOUND: usize = 50;
/// Trees that never hold projects and would swamp the walk.
const SKIPPED: [&str; 4] = ["node_modules", "target", "Library", ".git"];
/// Media folders directly in the home folder. They hold no projects, and reading the
/// Music and Photos libraries inside them makes macOS ask the owner for access.
const HOME_MEDIA: [&str; 3] = ["Music", "Movies", "Pictures"];
/// Package folders (apps, media libraries) are files to the owner; reading into one
/// can also make macOS ask for access.
const PACKAGES: [&str; 6] = [
    ".app",
    ".photoslibrary",
    ".musiclibrary",
    ".tvlibrary",
    ".imovielibrary",
    ".bundle",
];

/// Whether the walk leaves the folder `name`, found `depth` levels below the home
/// folder (0 for its own subfolders), unread.
fn skipped(name: &str, depth: usize) -> bool {
    name.starts_with('.')
        || SKIPPED.contains(&name)
        || (depth == 0 && HOME_MEDIA.contains(&name))
        || PACKAGES.iter().any(|suffix| name.ends_with(suffix))
}
/// Folders found by the walk reach the window in batches of this size.
pub const BATCH: usize = 500;

/// The subfolders of `folder` by name, hidden ones left out, links to folders followed,
/// at most [`MAX_LISTED`].
pub fn subfolders(folder: &Path) -> io::Result<Vec<String>> {
    let mut names: Vec<String> = std::fs::read_dir(folder)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            // `metadata` follows links, so a link to a folder counts as a folder.
            let folder = std::fs::metadata(entry.path()).ok()?.is_dir();
            (folder && !name.starts_with('.')).then_some(name)
        })
        .collect();
    names.sort_by_key(|name| name.to_lowercase());
    names.truncate(MAX_LISTED);
    Ok(names)
}

/// Walks the folders under `home` breadth first, skipping hidden folders and
/// [`SKIPPED`] trees and not following links, and hands them to `send` in batches. It
/// stops at [`WALK_DEPTH`] levels, [`WALK_LIMIT`] folders, or when `send` returns false.
pub fn walk(home: &Path, mut send: impl FnMut(Vec<PathBuf>) -> bool) {
    let mut queue = VecDeque::from([(home.to_path_buf(), 0usize)]);
    let mut batch = Vec::new();
    let mut seen = 0usize;
    while let Some((folder, depth)) = queue.pop_front() {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !kind.is_dir() || skipped(&name, depth) {
                continue;
            }
            let path = entry.path();
            if depth + 1 < WALK_DEPTH {
                queue.push_back((path.clone(), depth + 1));
            }
            batch.push(path);
            seen += 1;
            if seen >= WALK_LIMIT {
                send(batch);
                return;
            }
            if batch.len() >= BATCH && !send(std::mem::take(&mut batch)) {
                return;
            }
        }
    }
    if !batch.is_empty() {
        send(batch);
    }
}
