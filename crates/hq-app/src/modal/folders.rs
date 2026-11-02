//! The folder browser shared by both session windows (redesign-session-modal D6):
//! folder names only, read when shown and never stored.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub use hq_discovery::folders::{MAX_FOUND, subfolders, walk};

/// Where the browser reads folders: this computer, or the remote host on screen, whose
/// answers arrive later (add-remote-desktop-client D7).
#[derive(Clone, Default)]
pub enum Source {
    #[default]
    Local,
    Remote(Rc<RemoteFolders>),
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Local => f.write_str("Local"),
            Source::Remote(_) => f.write_str("Remote"),
        }
    }
}

impl PartialEq for Source {
    fn eq(&self, other: &Source) -> bool {
        match (self, other) {
            (Source::Local, Source::Local) => true,
            (Source::Remote(a), Source::Remote(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

impl Eq for Source {}

/// A folder's subfolders, or why it could not be read (`None`: it does not exist).
pub type FolderAnswer = Result<Vec<String>, Option<String>>;

/// A remote host's folders as far as it has answered, and the folders asked for since.
#[derive(Default)]
pub struct RemoteFolders {
    answers: RefCell<HashMap<PathBuf, FolderAnswer>>,
    wanted: RefCell<Vec<PathBuf>>,
}

impl RemoteFolders {
    /// Folders a listing needed that the host has not answered yet; each is asked once.
    pub fn take_wanted(&self) -> Vec<PathBuf> {
        std::mem::take(&mut *self.wanted.borrow_mut())
    }

    pub fn answer(&self, folder: PathBuf, answer: FolderAnswer) {
        self.answers.borrow_mut().insert(folder, answer);
    }

    fn ask(&self, folder: &Path) {
        let mut wanted = self.wanted.borrow_mut();
        if !wanted.iter().any(|known| known == folder) {
            wanted.push(folder.to_path_buf());
        }
    }
}

impl Source {
    fn subfolders(&self, folder: &Path) -> io::Result<Vec<String>> {
        match self {
            Source::Local => subfolders(folder),
            Source::Remote(remote) => {
                let answers = remote.answers.borrow();
                match answers.get(folder) {
                    Some(Ok(names)) => Ok(names.clone()),
                    Some(Err(None)) => Err(io::ErrorKind::NotFound.into()),
                    Some(Err(Some(reason))) => Err(io::Error::other(reason.clone())),
                    None => {
                        drop(answers);
                        remote.ask(folder);
                        Err(io::ErrorKind::WouldBlock.into())
                    }
                }
            }
        }
    }

    fn is_dir(&self, path: &Path) -> bool {
        match self {
            Source::Local => path.is_dir(),
            Source::Remote(remote) => {
                let known = remote.answers.borrow().get(path).map(Result::is_ok);
                match known {
                    Some(folder) => folder,
                    None => {
                        remote.ask(path);
                        false
                    }
                }
            }
        }
    }
}

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

/// `name` inside `parent`. On Windows the browser holds only a remote host's paths,
/// which are Unix paths, so they join with `/` there; `Path::join` would put `\`
/// between them and the host would refuse the path (add-windows-client D2).
fn child(parent: &Path, name: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!(
            "{}/{name}",
            parent.to_string_lossy().trim_end_matches('/')
        ))
    } else {
        parent.join(name)
    }
}

/// The folder `text` names, with `~` standing for the home folder.
pub fn expand(text: &str, home: &Path) -> PathBuf {
    let text = text.trim();
    if text == "~" {
        return home.to_path_buf();
    }
    match text.strip_prefix("~/") {
        Some(rest) => child(home, rest.trim_start_matches('/')),
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

/// The rows under the field for `text`. A path lists that folder and then its
/// subfolders, or, while its last part is half typed, the folders starting with it; a
/// bare name lists the matching folders from `found`, shortest first.
pub fn list(text: &str, home: &Path, found: &[PathBuf]) -> Listing {
    list_in(&Source::Local, text, home, found)
}

/// [`list`] reading folders from `source`.
pub fn list_in(source: &Source, text: &str, home: &Path, found: &[PathBuf]) -> Listing {
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
    if text.ends_with('/') || source.is_dir(&path) {
        return match source.subfolders(&path) {
            Ok(names) => Listing {
                rows: std::iter::once(FolderRow::Here(path.clone()))
                    .chain(
                        names
                            .into_iter()
                            .map(|name| FolderRow::Sub(child(&path, &name))),
                    )
                    .collect(),
                problem: None,
            },
            Err(error) if error.kind() == io::ErrorKind::NotFound => Listing {
                rows: Vec::new(),
                problem: Some(format!("No folder {}", shown(&path, home))),
            },
            // The remote host has not answered yet.
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => Listing {
                rows: vec![FolderRow::Here(path.clone())],
                problem: Some("Reading the host's folders…".to_owned()),
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
    let rows = source
        .subfolders(parent)
        .unwrap_or_default()
        .into_iter()
        .filter(|name| name.to_lowercase().starts_with(&start))
        .map(|name| FolderRow::Sub(child(parent, &name)))
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

#[cfg(test)]
mod tests {
    use std::fs;

    use hq_discovery::folders::{BATCH, MAX_LISTED, WALK_DEPTH};

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

    // This computer's folders: browsed only beside a local host, which runs on Unix.
    #[cfg(unix)]
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
    fn a_remote_hosts_folders_list_once_it_answers() {
        let remote = Rc::new(RemoteFolders::default());
        let source = Source::Remote(Rc::clone(&remote));
        let home = Path::new("/home/me");
        let waiting = list_in(&source, "~/", home, &[]);
        assert_eq!(
            waiting.problem.as_deref(),
            Some("Reading the host's folders…")
        );
        assert_eq!(remote.take_wanted(), [home.to_path_buf()]);
        assert!(remote.take_wanted().is_empty(), "asked once");
        remote.answer(home.to_path_buf(), Ok(vec!["code".into(), "notes".into()]));
        let listed = list_in(&source, "~/", home, &[]);
        assert_eq!(listed.problem, None);
        assert_eq!(
            listed.rows,
            [
                FolderRow::Here(home.to_path_buf()),
                FolderRow::Sub(home.join("code")),
                FolderRow::Sub(home.join("notes")),
            ]
        );
        remote.answer(home.join("gone"), Err(None));
        assert_eq!(
            list_in(&source, "~/gone/", home, &[]).problem.as_deref(),
            Some("No folder ~/gone")
        );
    }

    #[test]
    fn a_remote_hosts_paths_keep_their_slashes() {
        let remote = Rc::new(RemoteFolders::default());
        let source = Source::Remote(Rc::clone(&remote));
        let home = Path::new("/home/me");
        let project = expand("~/code/project", home);
        assert_eq!(project.to_string_lossy(), "/home/me/code/project");
        remote.answer(project.clone(), Ok(vec!["api".into()]));
        // Strings, not paths: Windows compares paths by component, `/` and `\` alike.
        let texts: Vec<String> = list_in(&source, "~/code/project/", home, &[])
            .rows
            .iter()
            .map(|row| match row {
                FolderRow::Here(path) | FolderRow::Sub(path) | FolderRow::Found(path) => {
                    path.to_string_lossy().into_owned()
                }
            })
            .collect();
        assert_eq!(
            texts,
            ["/home/me/code/project/", "/home/me/code/project/api"]
        );
        assert_eq!(into_text(Path::new(&texts[1]), home), "~/code/project/api/");
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

    // This computer's folders: browsed only beside a local host, which runs on Unix.
    #[cfg(unix)]
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

    // This computer's folders: browsed only beside a local host, which runs on Unix.
    #[cfg(unix)]
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
