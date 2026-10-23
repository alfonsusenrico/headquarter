//! The folder browser shared by both session windows (redesign-session-modal D6):
//! folder names only, read when shown and never stored.

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
const BATCH: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FolderRow {
    /// The typed folder itself, tagged "Open here".
    Here(PathBuf),
    /// A subfolder of the typed folder, or one whose name starts with the half-typed part.
    Sub(PathBuf),
    /// A folder found by name anywhere under the home folder.
    Found(PathBuf),
}

impl FolderRow {
    pub fn path(&self) -> &Path {
        match self {
            FolderRow::Here(path) | FolderRow::Sub(path) | FolderRow::Found(path) => path,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listing {
    pub rows: Vec<FolderRow>,
    /// Why the typed folder lists nothing, when it cannot be read.
    pub problem: Option<String>,
}

/// Text starting with `~` or `/` names a folder; anything else is a name to find.
pub fn is_path(text: &str) -> bool {
    let text = text.trim_start();
    text.starts_with('~') || text.starts_with('/')
}

/// The folder `text` names, with `~` standing for the home folder.
pub fn expand(text: &str, home: &Path) -> PathBuf {
    let text = text.trim();
    if text == "~" {
        return home.to_path_buf();
    }
    match text.strip_prefix("~/") {
        Some(rest) => home.join(rest.trim_start_matches('/')),
        None => PathBuf::from(text),
    }
}

/// How a folder is written in the field and the list: under the home folder as `~/…`.
pub fn shown(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
}

/// The field's text that goes into `path`.
pub fn into_text(path: &Path, home: &Path) -> String {
    let shown = shown(path, home);
    if shown.ends_with('/') {
        shown
    } else {
        format!("{shown}/")
    }
}

/// The field's text one folder up from the folder `text` names, if it has a parent.
pub fn up_text(text: &str, home: &Path) -> Option<String> {
    if !is_path(text) {
        return None;
    }
    let path = expand(text.trim().trim_end_matches('/'), home);
    let path = if path.as_os_str().is_empty() {
        PathBuf::from("/")
    } else {
        path
    };
    path.parent().map(|parent| into_text(parent, home))
}

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

/// The rows under the field for `text`. A path lists that folder and then its
/// subfolders, or, while its last part is half typed, the folders starting with it; a
/// bare name lists the matching folders from `found`, shortest first.
pub fn list(text: &str, home: &Path, found: &[PathBuf]) -> Listing {
    let text = text.trim();
    let text = if text.is_empty() { "~/" } else { text };
    if !is_path(text) {
        return Listing {
            rows: find(text, found)
                .into_iter()
                .map(|path| FolderRow::Found(path.clone()))
                .collect(),
            problem: None,
        };
    }
    let path = expand(text, home);
    if text.ends_with('/') || path.is_dir() {
        return match subfolders(&path) {
            Ok(names) => Listing {
                rows: std::iter::once(FolderRow::Here(path.clone()))
                    .chain(
                        names
                            .into_iter()
                            .map(|name| FolderRow::Sub(path.join(name))),
                    )
                    .collect(),
                problem: None,
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => Listing {
                rows: Vec::new(),
                problem: Some(format!("No folder {}", shown(&path, home))),
            },
            Err(error) => Listing {
                rows: vec![FolderRow::Here(path.clone())],
                problem: Some(format!("Cannot read {}: {error}", shown(&path, home))),
            },
        };
    }
    let (Some(parent), Some(start)) = (path.parent(), path.file_name()) else {
        return Listing::default();
    };
    let start = start.to_string_lossy().to_lowercase();
    let rows = subfolders(parent)
        .unwrap_or_default()
        .into_iter()
        .filter(|name| name.to_lowercase().starts_with(&start))
        .map(|name| FolderRow::Sub(parent.join(name)))
        .collect();
    Listing {
        rows,
        problem: None,
    }
}

/// The folders in `found` whose name contains `name`, ignoring case: exact names first,
/// then shortest paths, at most [`MAX_FOUND`].
pub fn find<'a>(name: &str, found: &'a [PathBuf]) -> Vec<&'a PathBuf> {
    let needle = name.trim().to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut matches: Vec<&PathBuf> = found
        .iter()
        .filter(|path| {
            path.file_name()
                .is_some_and(|last| last.to_string_lossy().to_lowercase().contains(&needle))
        })
        .collect();
    // A folder named exactly so comes before longer names that contain it.
    matches.sort_by_key(|path| {
        let exact = path
            .file_name()
            .is_some_and(|last| last.to_string_lossy().to_lowercase() == needle);
        (
            !exact,
            path.components().count(),
            path.as_os_str().len(),
            *path,
        )
    });
    matches.truncate(MAX_FOUND);
    matches
}

/// Where `name` matches the last part of `path`, as a byte range of `path`'s shown form.
pub fn match_range(shown: &str, name: &str) -> Option<std::ops::Range<usize>> {
    let needle = name.trim().to_lowercase();
    if needle.is_empty() {
        return None;
    }
    let last_start = shown.rfind('/').map_or(0, |slash| slash + 1);
    let last = &shown[last_start..];
    // Lowercasing can change byte lengths; only ASCII-preserving matches are marked.
    let lowered = last.to_lowercase();
    if lowered.len() != last.len() {
        return None;
    }
    let at = lowered.find(&needle)?;
    Some(last_start + at..last_start + at + needle.len())
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

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    /// A test folder, removed when dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Scratch {
            let dir =
                std::env::temp_dir().join(format!("hq-folders-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(
                    self.0.join("home/work/web"),
                    fs::Permissions::from_mode(0o755),
                );
            }
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// A home folder with projects, hidden folders and bulky trees.
    fn tree(name: &str) -> (Scratch, PathBuf) {
        let root = Scratch::new(name);
        let home = root.path().join("home");
        for folder in [
            "code/project/headquarter/src",
            "code/project/acme/team-flow-planner/app",
            "code/project/.git",
            "code/project/notes",
            "work/team-flow-planner",
            "work/web/node_modules/team-flow-planner",
            "Library/team-flow-planner",
            ".hidden/team-flow-planner",
        ] {
            fs::create_dir_all(home.join(folder)).unwrap();
        }
        fs::write(home.join("code/project/readme.md"), "a file").unwrap();
        (root, home)
    }

    fn walked(home: &Path) -> Vec<PathBuf> {
        let mut all = Vec::new();
        walk(home, |batch| {
            all.extend(batch);
            true
        });
        all
    }

    fn shown_rows(listing: &Listing, home: &Path) -> Vec<String> {
        listing
            .rows
            .iter()
            .map(|row| {
                let tag = match row {
                    FolderRow::Here(_) => "here ",
                    FolderRow::Sub(_) => "",
                    FolderRow::Found(_) => "found ",
                };
                format!("{tag}{}", shown(row.path(), home))
            })
            .collect()
    }

    #[test]
    fn a_path_lists_the_folder_then_its_visible_subfolders() {
        let (_root, home) = tree("path");
        let listing = list("~/code/project/", &home, &[]);
        assert_eq!(
            shown_rows(&listing, &home),
            [
                "here ~/code/project",
                "~/code/project/acme",
                "~/code/project/headquarter",
                "~/code/project/notes",
            ],
            "the folder first, then subfolders by name; .git and files left out"
        );
        assert_eq!(
            list("~/code/project", &home, &[]),
            listing,
            "an existing folder lists the same without its slash"
        );
    }

    #[test]
    fn a_half_typed_name_lists_the_folders_that_start_with_it() {
        let (_root, home) = tree("half");
        let listing = list("~/code/project/HE", &home, &[]);
        assert_eq!(shown_rows(&listing, &home), ["~/code/project/headquarter"]);
        assert!(list("~/code/project/zz", &home, &[]).rows.is_empty());
    }

    #[test]
    fn an_empty_field_lists_the_home_folder() {
        let (_root, home) = tree("empty");
        let rows = shown_rows(&list("", &home, &[]), &home);
        assert_eq!(rows, ["here ~", "~/code", "~/Library", "~/work"]);
    }

    #[test]
    fn a_missing_folder_says_so() {
        let (_root, home) = tree("missing");
        let listing = list("~/gone/", &home, &[]);
        assert!(listing.rows.is_empty());
        assert_eq!(listing.problem.as_deref(), Some("No folder ~/gone"));
    }

    #[cfg(unix)]
    #[test]
    fn an_unreadable_folder_says_so_and_can_still_be_chosen() {
        use std::os::unix::fs::PermissionsExt;

        let (_root, home) = tree("locked");
        let locked = home.join("work/web");
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
        let listing = list("~/work/web/", &home, &[]);
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        if listing.problem.is_none() {
            // Running as root reads every folder.
            return;
        }
        assert_eq!(listing.rows, [FolderRow::Here(locked)]);
        assert!(
            listing
                .problem
                .unwrap()
                .starts_with("Cannot read ~/work/web")
        );
    }

    #[cfg(unix)]
    #[test]
    fn links_to_folders_are_listed() {
        let (_root, home) = tree("links");
        std::os::unix::fs::symlink(home.join("work"), home.join("code/project/linked")).unwrap();
        let rows = shown_rows(&list("~/code/project/", &home, &[]), &home);
        assert!(rows.contains(&"~/code/project/linked".to_owned()));
    }

    #[test]
    fn listing_stops_at_the_limit() {
        let root = Scratch::new("limit");
        for index in 0..MAX_LISTED + 5 {
            fs::create_dir(root.path().join(format!("f{index:03}"))).unwrap();
        }
        assert_eq!(subfolders(root.path()).unwrap().len(), MAX_LISTED);
    }

    #[test]
    fn a_bare_name_finds_folders_anywhere_shortest_first() {
        let (_root, home) = tree("found");
        let found = walked(&home);
        let listing = list("team-flow", &home, &found);
        assert_eq!(
            shown_rows(&listing, &home),
            [
                "found ~/work/team-flow-planner",
                "found ~/code/project/acme/team-flow-planner",
            ],
            "hidden folders, node_modules and Library are not walked"
        );
    }

    #[test]
    fn the_walk_leaves_media_libraries_and_packages_unread() {
        let root = Scratch::new("media");
        for folder in [
            "Music/Music/Media.localized",
            "Pictures/Photos Library.photoslibrary/originals",
            "Movies/clips",
            "Applications/Tool.app/Contents",
            "code/site/Music",
            "code/site/Pictures",
        ] {
            fs::create_dir_all(root.path().join(folder)).unwrap();
        }
        let found = walked(root.path());
        let shown: Vec<String> = found
            .iter()
            .map(|path| {
                path.strip_prefix(root.path())
                    .unwrap()
                    .display()
                    .to_string()
            })
            .collect();
        for unread in ["Music", "Pictures", "Movies", "Applications/Tool.app"] {
            assert!(
                !shown.iter().any(|path| path.starts_with(unread)),
                "{unread} was read: {shown:?}"
            );
        }
        assert!(
            shown.contains(&"code/site/Music".to_owned()),
            "a project's own Music folder is still found: {shown:?}"
        );
        assert!(shown.contains(&"Applications".to_owned()));
    }

    #[test]
    fn the_walk_stops_at_the_depth_limit_and_skips_links() {
        let root = Scratch::new("deep");
        let deep: PathBuf = (1..=WALK_DEPTH + 2)
            .map(|level| format!("d{level}"))
            .collect();
        fs::create_dir_all(root.path().join(&deep)).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.path().join("d1"), root.path().join("d1/loop")).unwrap();
        let found = walked(root.path());
        assert_eq!(found.len(), WALK_DEPTH, "{found:?}");
        assert!(found.iter().all(|path| !path.ends_with("loop")));
    }

    #[test]
    fn the_walk_stops_when_the_window_closes() {
        let root = Scratch::new("stop");
        for index in 0..BATCH * 3 {
            fs::create_dir(root.path().join(format!("f{index}"))).unwrap();
        }
        let mut batches = 0;
        walk(root.path(), |_| {
            batches += 1;
            false
        });
        assert_eq!(batches, 1);
    }

    #[test]
    fn an_exact_name_comes_first() {
        let found = vec![
            PathBuf::from("/h/a/terminal_client"),
            PathBuf::from("/h/a/terminal"),
            PathBuf::from("/h/a/b/c/terminal"),
        ];
        assert_eq!(find("Terminal", &found), [&found[1], &found[2], &found[0]]);
    }

    #[test]
    fn found_folders_are_capped() {
        let found: Vec<PathBuf> = (0..MAX_FOUND + 10)
            .map(|index| PathBuf::from(format!("/h/p{index}/app")))
            .collect();
        assert_eq!(find("app", &found).len(), MAX_FOUND);
    }

    #[test]
    fn moving_in_and_out_of_folders() {
        let home = Path::new("/Users/me");
        assert_eq!(
            into_text(Path::new("/Users/me/code/project"), home),
            "~/code/project/"
        );
        assert_eq!(into_text(home, home), "~/");
        assert_eq!(up_text("~/code/project/", home).as_deref(), Some("~/code/"));
        assert_eq!(up_text("~/code", home).as_deref(), Some("~/"));
        assert_eq!(up_text("~/", home).as_deref(), Some("/Users/"));
        assert_eq!(up_text("/", home), None);
        assert_eq!(up_text("cash", home), None);
    }

    #[test]
    fn the_match_is_marked_in_the_last_part() {
        assert_eq!(
            match_range("~/work/team-flow-planner", "FLOW"),
            Some(12..16)
        );
        assert_eq!(match_range("~/work/web", "work"), None);
    }
}
