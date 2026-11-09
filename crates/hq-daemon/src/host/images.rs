//! Images sent to a session (add-image-upload): checked by their first bytes, saved under
//! the data folder in a folder per session, readable by the owner only, and deleted with the
//! session or 7 days after they arrived. The log never names an image or its path (D8).

use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use hq_protocol::{ErrorCode, Response};

use hq_signals::now_ms;

use super::{Host, OpResult, fail, lock};

/// The largest image the host takes: 10 MiB (D3).
pub const MAX_BYTES: u64 = 10 * 1024 * 1024;
/// How long an image is kept while its session stays in the sidebar (owner, 2026-11-06).
pub const KEPT_FOR: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// How often the host deletes what is past keeping.
pub const SWEEP_EVERY: Duration = Duration::from_secs(60 * 60);
/// A partial file left by a host that stopped mid-upload is deleted after this long.
const PART_KEPT_FOR: Duration = Duration::from_secs(60 * 60);
const PART: &str = "part";

/// Files saved by this host process, so two images in one millisecond get their own names.
static SAVED: AtomicU64 = AtomicU64::new(0);

/// The image formats Claude Code and Codex read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Png,
    Jpeg,
    Gif,
    Webp,
}

impl Kind {
    /// The format the file's first bytes show, whatever the client said (D3).
    pub fn of(head: &[u8]) -> Option<Kind> {
        if head.starts_with(b"\x89PNG\r\n\x1a\n") {
            Some(Kind::Png)
        } else if head.starts_with(b"\xff\xd8\xff") {
            Some(Kind::Jpeg)
        } else if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
            Some(Kind::Gif)
        } else if head.len() >= 12 && head.starts_with(b"RIFF") && &head[8..12] == b"WEBP" {
            Some(Kind::Webp)
        } else {
            None
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Kind::Png => "png",
            Kind::Jpeg => "jpeg",
            Kind::Gif => "gif",
            Kind::Webp => "webp",
        }
    }

    fn extension(self) -> &'static str {
        match self {
            Kind::Png => "png",
            Kind::Jpeg => "jpg",
            Kind::Gif => "gif",
            Kind::Webp => "webp",
        }
    }
}

/// Why an image was not saved. Each says what the owner can do.
#[derive(Debug)]
pub enum Refusal {
    TooLarge,
    NotAnImage,
    /// More bytes than announced.
    TooMuch,
    Storage(std::io::Error),
}

impl Refusal {
    pub fn message(&self) -> String {
        match self {
            Refusal::TooLarge => "The image is larger than 10 MB".to_owned(),
            Refusal::NotAnImage => "Only PNG, JPEG, GIF or WebP images can be sent".to_owned(),
            Refusal::TooMuch => "The image sent more bytes than it announced".to_owned(),
            Refusal::Storage(error) => format!("The image could not be saved: {error}"),
        }
    }

    /// The reason as the log names it.
    pub fn reason(&self) -> &'static str {
        match self {
            Refusal::TooLarge => "size",
            Refusal::NotAnImage => "kind",
            Refusal::TooMuch => "too much",
            Refusal::Storage(_) => "storage",
        }
    }
}

/// One image arriving: written to a `.part` file, renamed once whole and checked. Dropped
/// before that, the partial file is removed (D2).
pub struct Upload {
    session: String,
    folder: PathBuf,
    part: PathBuf,
    stamp: u64,
    file: Option<File>,
    remaining: u64,
    size: u64,
    head: Vec<u8>,
}

/// What `Upload::write` leaves to do.
#[derive(Debug, PartialEq, Eq)]
pub enum Progress {
    More,
    Whole,
}

impl Upload {
    /// Starts an image of `size` bytes for the session with index id `session` (never a
    /// client's own reference: the id names the folder).
    pub fn start(root: &Path, session: &str, size: u64, now_ms: u64) -> Result<Upload, Refusal> {
        if size > MAX_BYTES {
            return Err(Refusal::TooLarge);
        }
        if size == 0 {
            return Err(Refusal::NotAnImage);
        }
        let folder = root.join(session);
        private_folder(root).map_err(Refusal::Storage)?;
        private_folder(&folder).map_err(Refusal::Storage)?;
        let stamp = now_ms;
        let count = SAVED.fetch_add(1, Ordering::Relaxed);
        let part = folder.join(format!("{stamp}-{count}.{PART}"));
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&part)
            .map_err(Refusal::Storage)?;
        Ok(Upload {
            session: session.to_owned(),
            folder,
            part,
            stamp,
            file: Some(file),
            remaining: size,
            size,
            head: Vec::with_capacity(12),
        })
    }

    /// The index id of the session the image is for.
    pub fn session(&self) -> &str {
        &self.session
    }

    /// Writes the next bytes; `Whole` once the announced size has arrived.
    pub fn write(&mut self, bytes: &[u8]) -> Result<Progress, Refusal> {
        let length = bytes.len() as u64;
        if length > self.remaining {
            return Err(Refusal::TooMuch);
        }
        if self.head.len() < 12 {
            let wanted = (12 - self.head.len()).min(bytes.len());
            self.head.extend_from_slice(&bytes[..wanted]);
        }
        let Some(file) = self.file.as_mut() else {
            return Err(Refusal::TooMuch);
        };
        file.write_all(bytes).map_err(Refusal::Storage)?;
        self.remaining -= length;
        Ok(if self.remaining == 0 {
            Progress::Whole
        } else {
            Progress::More
        })
    }

    /// Checks the whole image's format and gives it its final name.
    pub fn finish(mut self) -> Result<Saved, Refusal> {
        let kind = Kind::of(&self.head).ok_or(Refusal::NotAnImage)?;
        let Some(file) = self.file.take() else {
            return Err(Refusal::TooMuch);
        };
        file.sync_all().map_err(Refusal::Storage)?;
        drop(file);
        let name = self
            .part
            .file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_else(|| self.stamp.to_string());
        let path = self.folder.join(format!("{name}.{}", kind.extension()));
        std::fs::rename(&self.part, &path).map_err(Refusal::Storage)?;
        Ok(Saved {
            path,
            kind,
            bytes: self.size,
        })
    }
}

impl Drop for Upload {
    fn drop(&mut self) {
        // Renamed when finished; otherwise the partial image goes.
        let _ = std::fs::remove_file(&self.part);
    }
}

/// An image the host saved.
#[derive(Debug)]
pub struct Saved {
    pub path: PathBuf,
    pub kind: Kind,
    pub bytes: u64,
}

impl Host {
    pub(super) fn images_root(&self) -> PathBuf {
        self.paths.dir.join("images")
    }

    /// Starts an image for a session in the sidebar, on a connection attached to nothing (D1, D3).
    pub(super) fn start_upload(
        &self,
        reference: &str,
        size: u64,
        attached: bool,
    ) -> OpResult<Upload> {
        if attached {
            return Err(fail(
                ErrorCode::BadRequest,
                "An image is sent on a connection of its own",
            ));
        }
        let row = self
            .resolve(reference)
            .ok()
            .filter(|row| row.sidebar_added_at.is_some())
            .ok_or_else(|| fail(ErrorCode::NotFound, "The session is no longer there"))?;
        Upload::start(&self.images_root(), &row.id, size, now_ms())
            .map_err(|refusal| self.refused_upload(&row.id, &refusal))
    }

    /// The answer once the image's last byte arrived.
    pub(super) fn finish_upload(&self, upload: Upload) -> Response {
        let session = upload.session().to_owned();
        match upload.finish() {
            Ok(saved) => {
                self.log.line(&format!(
                    "image saved session={session} bytes={} kind={}",
                    saved.bytes,
                    saved.kind.name()
                ));
                Response::Uploaded {
                    path: saved.path.to_string_lossy().into_owned(),
                }
            }
            Err(refusal) => *self.refused_upload(&session, &refusal),
        }
    }

    pub(super) fn refused_upload(&self, session: &str, refusal: &Refusal) -> Box<Response> {
        self.log.line(&format!(
            "image refused session={session} reason={}",
            refusal.reason()
        ));
        let code = match refusal {
            Refusal::TooLarge => ErrorCode::TooLarge,
            Refusal::Storage(_) => ErrorCode::Storage,
            Refusal::NotAnImage | Refusal::TooMuch => ErrorCode::BadRequest,
        };
        fail(code, refusal.message())
    }

    /// An upload another request interrupted, or whose connection ended: nothing is kept.
    pub(super) fn cut_short(&self, upload: Upload) {
        self.log.line(&format!(
            "image refused session={} reason=cut short",
            upload.session()
        ));
    }

    /// Deletes a session's images as it leaves the sidebar (D4).
    pub(super) fn delete_images(&self, session: &str) {
        let count = delete_session(&self.images_root(), session);
        if count > 0 {
            self.log
                .line(&format!("images deleted session={session} count={count}"));
        }
    }

    /// Deletes images past keeping and those of sessions no longer in the sidebar (D4).
    pub(super) fn sweep_images(&self) {
        // Without the index nothing can be told apart, so nothing is deleted.
        let Ok(rows) = lock(&self.store).list() else {
            return;
        };
        let kept: HashSet<String> = rows
            .into_iter()
            .filter(|row| row.sidebar_added_at.is_some())
            .map(|row| row.id)
            .collect();
        let count = sweep(&self.images_root(), now_ms(), |session| {
            kept.contains(session)
        });
        if count > 0 {
            self.log.line(&format!("images deleted count={count}"));
        }
    }

    pub(super) fn images_loop(self: Arc<Self>) {
        loop {
            thread::sleep(SWEEP_EVERY);
            if self.stopping.load(Ordering::SeqCst) {
                return;
            }
            self.sweep_images();
        }
    }
}

/// Deletes a session's images with the session (D4). Returns how many files went.
pub fn delete_session(root: &Path, session: &str) -> usize {
    let folder = root.join(session);
    let count = files_in(&folder).len();
    if folder.is_dir() && std::fs::remove_dir_all(&folder).is_ok() {
        count
    } else {
        0
    }
}

/// Deletes images older than `KEPT_FOR` and the folders of sessions `kept` does not name
/// (D4). Returns how many files went.
pub fn sweep(root: &Path, now_ms: u64, kept: impl Fn(&str) -> bool) -> usize {
    let Ok(entries) = std::fs::read_dir(root) else {
        return 0;
    };
    let mut deleted = 0;
    for entry in entries.flatten() {
        let folder = entry.path();
        if !folder.is_dir() {
            continue;
        }
        let session = entry.file_name().to_string_lossy().into_owned();
        if !kept(&session) {
            deleted += delete_session(root, &session);
            continue;
        }
        for file in files_in(&folder) {
            if past_keeping(&file, now_ms) && std::fs::remove_file(&file).is_ok() {
                deleted += 1;
            }
        }
    }
    deleted
}

/// Whether an image (or a partial one) is older than it may be kept, by the time in its name.
/// A file whose name has no time is not Headquarter's and is left alone.
fn past_keeping(file: &Path, now_ms: u64) -> bool {
    let Some(name) = file
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
    else {
        return false;
    };
    let Some(stamp) = name
        .split(['-', '.'])
        .next()
        .and_then(|stamp| stamp.parse::<u64>().ok())
    else {
        return false;
    };
    let kept_for = if name.ends_with(&format!(".{PART}")) {
        PART_KEPT_FOR
    } else {
        KEPT_FOR
    };
    now_ms.saturating_sub(stamp) > kept_for.as_millis() as u64
}

fn files_in(folder: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .map(|entry| entry.path())
                .filter(|path| path.is_file())
                .collect()
        })
        .unwrap_or_default()
}

fn private_folder(folder: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(folder)?;
    std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    const NOW: u64 = 1_800_000_000_000;

    fn root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("hq-images-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }

    #[test]
    fn formats_are_read_from_the_first_bytes() {
        assert_eq!(Kind::of(PNG), Some(Kind::Png));
        assert_eq!(Kind::of(b"\xff\xd8\xff\xe0\0\x10JFIF"), Some(Kind::Jpeg));
        assert_eq!(Kind::of(b"GIF89a\x01\0"), Some(Kind::Gif));
        assert_eq!(Kind::of(b"RIFF\x24\0\0\0WEBPVP8 "), Some(Kind::Webp));
        assert_eq!(Kind::of(b"%PDF-1.7\n"), None);
        assert_eq!(Kind::of(b"RIFF\x24\0\0\0WAVE"), None);
        assert_eq!(Kind::of(b""), None);
    }

    #[test]
    fn an_image_is_saved_private_under_its_session() {
        let root = root("saved");
        let mut image = PNG.to_vec();
        image.extend_from_slice(&[7; 100]);
        let mut upload = Upload::start(&root, "c4f1", image.len() as u64, NOW).unwrap();
        assert_eq!(upload.write(&image[..10]).unwrap(), Progress::More);
        assert_eq!(upload.write(&image[10..]).unwrap(), Progress::Whole);
        let saved = upload.finish().unwrap();

        assert_eq!(saved.kind, Kind::Png);
        assert_eq!(saved.path.parent().unwrap(), root.join("c4f1"));
        assert!(saved.path.to_string_lossy().ends_with(".png"));
        assert_eq!(std::fs::read(&saved.path).unwrap(), image);
        let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&saved.path), 0o600);
        assert_eq!(mode(&root.join("c4f1")), 0o700);
        assert_eq!(mode(&root), 0o700);
        assert_eq!(
            files_in(&root.join("c4f1")).len(),
            1,
            "no partial file left"
        );
    }

    #[test]
    fn more_than_ten_megabytes_is_refused_before_anything_is_written() {
        let root = root("large");
        assert!(matches!(
            Upload::start(&root, "c4f1", MAX_BYTES + 1, NOW),
            Err(Refusal::TooLarge)
        ));
        assert!(!root.join("c4f1").exists());
        assert!(Upload::start(&root, "c4f1", MAX_BYTES, NOW).is_ok());
    }

    #[test]
    fn something_else_than_an_image_is_refused_and_removed() {
        let root = root("pdf");
        let pdf = b"%PDF-1.7\n%stuff";
        let mut upload = Upload::start(&root, "c4f1", pdf.len() as u64, NOW).unwrap();
        upload.write(pdf).unwrap();
        assert!(matches!(upload.finish(), Err(Refusal::NotAnImage)));
        assert!(files_in(&root.join("c4f1")).is_empty());
    }

    #[test]
    fn a_cut_short_upload_leaves_nothing() {
        let root = root("cut");
        let mut upload = Upload::start(&root, "c4f1", 100, NOW).unwrap();
        upload.write(PNG).unwrap();
        drop(upload);
        assert!(files_in(&root.join("c4f1")).is_empty());
    }

    #[test]
    fn more_bytes_than_announced_are_refused() {
        let root = root("much");
        let mut upload = Upload::start(&root, "c4f1", 4, NOW).unwrap();
        assert!(matches!(upload.write(PNG), Err(Refusal::TooMuch)));
    }

    #[test]
    fn images_go_with_their_session_and_after_a_week() {
        let root = root("sweep");
        let day = 24 * 60 * 60 * 1000;
        let save = |session: &str, stamp: u64| {
            let mut upload = Upload::start(&root, session, PNG.len() as u64, stamp).unwrap();
            upload.write(PNG).unwrap();
            upload.finish().unwrap().path
        };
        let old = save("kept", NOW - 8 * day);
        let fresh = save("kept", NOW - day);
        let gone = save("closed", NOW);
        let stale_part = root.join("kept").join(format!("{}-9.part", NOW - 2 * day));
        std::fs::write(&stale_part, b"x").unwrap();
        let foreign = root.join("kept").join("notes.txt");
        std::fs::write(&foreign, b"x").unwrap();

        assert_eq!(sweep(&root, NOW, |session| session == "kept"), 3);
        assert!(!old.exists());
        assert!(fresh.exists());
        assert!(!gone.exists() && !root.join("closed").exists());
        assert!(!stale_part.exists());
        assert!(foreign.exists(), "not Headquarter's file");

        assert_eq!(delete_session(&root, "kept"), 2);
        assert!(!root.join("kept").exists());
    }
}
