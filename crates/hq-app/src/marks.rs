//! Agents' marks: each vendor's own published file, fetched once onto the owner's
//! machine and drawn unaltered. Nothing is bundled, so nothing is redistributed
//! (`openspec/changes/redesign-session-modal/design.md`, D5). With `agent_marks =
//! "mono"` (the default), a vendor's own one-colour mark is drawn in black or white
//! against the theme; a vendor that publishes none keeps its coloured mark. Until a
//! mark is present, or with `agent_marks = "drawn"`, Headquarter's drawn mark stands in.

use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use alacritty_terminal::vte::ansi::Rgb;
use gpui::{AnyElement, IntoElement, ParentElement, Styled, div, img, px, svg};
use hq_discovery::MarkSource;
use hq_signals::event::Agent;

use crate::sidebar::glyph;
use crate::terminal::view::hsla;
use crate::theme::Paint;

/// Larger marks are refused and nothing is kept.
const MAX_BYTES: u64 = 512 * 1024;
/// Larger downloads (a vendor's zipped logo pack included) are refused.
const MAX_DOWNLOAD: &str = "4000000";
/// The fetch gives up after this many seconds, leaving the drawn mark.
const TIMEOUT_SECS: &str = "20";

/// Which of a vendor's marks to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// The one-colour mark where the vendor publishes one, else the coloured one.
    Mono,
    Color,
}

#[derive(Debug, Clone, PartialEq)]
enum Kept {
    Color(PathBuf),
    /// A one-colour SVG, drawn in the ink colour.
    Mono(PathBuf),
}

/// The marks present on this machine, and the ink one-colour marks are drawn in.
#[derive(Debug, Clone, PartialEq)]
pub struct Marks {
    kept: HashMap<Agent, Kept>,
    ink: Rgb,
}

impl Default for Marks {
    fn default() -> Marks {
        Marks {
            kept: HashMap::new(),
            ink: Rgb { r: 0, g: 0, b: 0 },
        }
    }
}

impl Marks {
    /// The marks of `style` already fetched into `dir`.
    pub fn load(dir: &Path, style: Style) -> Marks {
        let kept = Agent::ALL
            .into_iter()
            .filter_map(|agent| {
                let wanted = wanted(dir, agent, style)?;
                if !wanted.path.is_file() {
                    return None;
                }
                let kept = if wanted.mono {
                    Kept::Mono(wanted.path)
                } else {
                    Kept::Color(wanted.path)
                };
                Some((agent, kept))
            })
            .collect();
        Marks {
            kept,
            ..Marks::default()
        }
    }

    /// One-colour marks are drawn in black on a light theme and white on a dark one,
    /// as vendors publish them.
    pub fn set_ink(&mut self, ink: Rgb) {
        self.ink = ink;
    }

    /// The agent's mark at `size` pixels: the vendor's file, or the drawn mark in
    /// `color`.
    pub fn element(&self, agent: Agent, size: f32, color: impl Into<Paint>) -> AnyElement {
        let color = color.into();
        let cell = div()
            .size(px(size))
            .flex_none()
            .flex()
            .items_center()
            .justify_center();
        match self.kept.get(&agent) {
            Some(Kept::Color(path)) => cell
                .child(img(path.clone()).size(px(size - 3.)))
                .into_any_element(),
            Some(Kept::Mono(path)) => cell
                .child(
                    svg()
                        .external_path(path.to_string_lossy().into_owned())
                        .size(px(size - 3.))
                        .text_color(hsla(self.ink)),
                )
                .into_any_element(),
            None => glyph::agent(agent, size, color),
        }
    }

    pub fn has(&self, agent: Agent) -> bool {
        self.kept.contains_key(&agent)
    }
}

/// `<data folder>/marks`.
pub fn dir() -> Option<PathBuf> {
    hq_signals::paths::data_dir().map(|data| data.join("marks"))
}

struct Wanted {
    source: MarkSource,
    path: PathBuf,
    mono: bool,
}

/// The mark to show for `style` and where it is kept: `<id>.<ext>`, or
/// `<id>-mono.svg` for a one-colour mark.
fn wanted(dir: &Path, agent: Agent, style: Style) -> Option<Wanted> {
    let known = hq_discovery::known(agent.id())?;
    let mono = known.mark_mono.filter(|_| style == Style::Mono);
    let (source, suffix) = match mono {
        Some(source) => (source, "-mono"),
        None => (known.mark_color?, ""),
    };
    let named = source.entry.unwrap_or(source.url);
    let extension = Path::new(named).extension()?.to_str()?;
    Some(Wanted {
        source,
        path: dir.join(format!("{}{suffix}.{extension}", agent.id())),
        mono: mono.is_some(),
    })
}

/// Fetches every hostable agent's mark of `style` that is not on this machine yet, one
/// HTTPS GET each with `curl`, and returns the marks then present. A failure leaves
/// that agent with its drawn mark and is tried again the next time the app starts.
pub fn fetch_missing(dir: &Path, style: Style) -> Marks {
    fetch_missing_with(dir, style, &curl)
}

fn fetch_missing_with(
    dir: &Path,
    style: Style,
    fetch: &dyn Fn(&str, &Path) -> Result<(), String>,
) -> Marks {
    for agent in Agent::ALL {
        let Some(wanted) = wanted(dir, agent, style) else {
            continue;
        };
        if wanted.path.is_file() {
            continue;
        }
        if let Err(problem) = fetch_one(&wanted, fetch) {
            eprintln!("headquarter-app: the {agent} mark was not fetched: {problem}");
        }
    }
    Marks::load(dir, style)
}

/// Fetches into a temporary file (taking the one entry from a zip archive), checks it,
/// then moves it into place, so a half-written or wrong file is never shown.
fn fetch_one(
    wanted: &Wanted,
    fetch: &dyn Fn(&str, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let dir = wanted.path.parent().ok_or("no folder for the mark")?;
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let partial = wanted.path.with_extension("part");
    let download = wanted.path.with_extension("download");
    let result = fetch(wanted.source.url, &download)
        .and_then(|()| match wanted.source.entry {
            None => std::fs::rename(&download, &partial).map_err(|error| error.to_string()),
            Some(entry) => {
                let archive = std::fs::read(&download).map_err(|error| error.to_string())?;
                let file = zip_entry(&archive, entry)?;
                std::fs::write(&partial, file).map_err(|error| error.to_string())
            }
        })
        .and_then(|()| check(&partial, wanted.mono));
    let _ = std::fs::remove_file(&download);
    match result {
        Ok(()) => std::fs::rename(&partial, &wanted.path).map_err(|error| error.to_string()),
        Err(problem) => {
            let _ = std::fs::remove_file(&partial);
            Err(problem)
        }
    }
}

/// A PNG or SVG of a sensible size; a one-colour mark must be an SVG.
fn check(path: &Path, mono: bool) -> Result<(), String> {
    let size = std::fs::metadata(path)
        .map_err(|error| error.to_string())?
        .len();
    if size == 0 || size > MAX_BYTES {
        return Err(format!("unexpected size of {size} bytes"));
    }
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let png = bytes.starts_with(b"\x89PNG\r\n\x1a\n");
    let svg = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).contains("<svg");
    match (png, svg, mono) {
        (_, true, _) | (true, false, false) => Ok(()),
        _ => Err("not a PNG or SVG image".to_owned()),
    }
}

/// The contents of `name` in a zip archive, found through the central directory and
/// stored or deflated, as logo packs are.
fn zip_entry(archive: &[u8], name: &str) -> Result<Vec<u8>, String> {
    let u16_at = |at: usize| -> Option<usize> {
        Some(u16::from_le_bytes(archive.get(at..at + 2)?.try_into().ok()?).into())
    };
    let u32_at = |at: usize| -> Option<usize> {
        usize::try_from(u32::from_le_bytes(
            archive.get(at..at + 4)?.try_into().ok()?,
        ))
        .ok()
    };
    let broken = || "not a readable zip archive".to_owned();
    // The end-of-central-directory record is in the last 64 KB plus its own 22 bytes.
    let end = (archive.len().saturating_sub(65_557)..archive.len().saturating_sub(21))
        .rev()
        .find(|at| archive[*at..].starts_with(b"PK\x05\x06"))
        .ok_or_else(broken)?;
    let count = u16_at(end + 10).ok_or_else(broken)?;
    let mut at = u32_at(end + 16).ok_or_else(broken)?;
    for _ in 0..count {
        if !archive
            .get(at..)
            .is_some_and(|rest| rest.starts_with(b"PK\x01\x02"))
        {
            return Err(broken());
        }
        let method = u16_at(at + 10).ok_or_else(broken)?;
        let packed = u32_at(at + 20).ok_or_else(broken)?;
        let unpacked = u32_at(at + 24).ok_or_else(broken)?;
        let name_len = u16_at(at + 28).ok_or_else(broken)?;
        let extra_len = u16_at(at + 30).ok_or_else(broken)?;
        let comment_len = u16_at(at + 32).ok_or_else(broken)?;
        let local = u32_at(at + 42).ok_or_else(broken)?;
        let entry = archive
            .get(at + 46..at + 46 + name_len)
            .ok_or_else(broken)?;
        at += 46 + name_len + extra_len + comment_len;
        if entry != name.as_bytes() {
            continue;
        }
        if u64::try_from(unpacked).unwrap_or(u64::MAX) > MAX_BYTES {
            return Err(format!("`{name}` is too large"));
        }
        let start = local
            + 30
            + u16_at(local + 26).ok_or_else(broken)?
            + u16_at(local + 28).ok_or_else(broken)?;
        let data = archive.get(start..start + packed).ok_or_else(broken)?;
        return match method {
            0 => Ok(data.to_vec()),
            8 => {
                let mut out = Vec::with_capacity(unpacked);
                flate2::read::DeflateDecoder::new(data)
                    .take(MAX_BYTES)
                    .read_to_end(&mut out)
                    .map_err(|error| error.to_string())?;
                Ok(out)
            }
            other => Err(format!("`{name}` uses compression method {other}")),
        };
    }
    Err(format!("`{name}` is not in the archive"))
}

/// One HTTPS GET; redirects may not leave HTTPS. `--compressed` because a server may
/// send a compressed body unasked (antigravity.google did, 2026-10-20), and curl then
/// unpacks it.
fn curl(url: &str, out: &Path) -> Result<(), String> {
    let status = Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--compressed",
        ])
        .args(["--proto", "=https", "--proto-redir", "=https"])
        .args(["--max-time", TIMEOUT_SECS, "--max-filesize", MAX_DOWNLOAD])
        .arg("--output")
        .arg(out)
        .arg(url)
        .status()
        .map_err(|error| format!("cannot run curl: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("curl exited with {status}"))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io::Write;

    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest";
    const SVG: &[u8] = b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>";

    /// A path under the system's temporary folder, removed when the test ends.
    struct TempPath(std::path::PathBuf);

    impl std::ops::Deref for TempPath {
        type Target = std::path::Path;

        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl AsRef<std::path::Path> for TempPath {
        fn as_ref(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for TempPath {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
            let _ = std::fs::remove_file(&self.0);
        }
    }

    fn folder(name: &str) -> TempPath {
        let dir = std::env::temp_dir().join(format!("hq-marks-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        TempPath(dir)
    }

    /// A zip archive holding `files` deflated, as a logo pack is.
    fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, body) in files {
            let mut encoder =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(body).unwrap();
            let packed = encoder.finish().unwrap();
            let local = u32::try_from(out.len()).unwrap();
            let sizes = |buffer: &mut Vec<u8>| {
                buffer.extend(0u32.to_le_bytes()); // checksum, not checked
                buffer.extend(u32::try_from(packed.len()).unwrap().to_le_bytes());
                buffer.extend(u32::try_from(body.len()).unwrap().to_le_bytes());
                buffer.extend(u16::try_from(name.len()).unwrap().to_le_bytes());
                buffer.extend(0u16.to_le_bytes()); // extra field length
            };
            out.extend(b"PK\x03\x04");
            out.extend([20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
            sizes(&mut out);
            out.extend(name.as_bytes());
            out.extend(&packed);
            central.extend(b"PK\x01\x02");
            central.extend([20, 0, 20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
            sizes(&mut central);
            central.extend([0; 10]); // comment length, disk, attributes
            central.extend(local.to_le_bytes());
            central.extend(name.as_bytes());
        }
        let start = u32::try_from(out.len()).unwrap();
        let entries = u16::try_from(files.len()).unwrap();
        out.extend(&central);
        out.extend(b"PK\x05\x06");
        out.extend([0, 0, 0, 0]);
        out.extend(entries.to_le_bytes());
        out.extend(entries.to_le_bytes());
        out.extend(u32::try_from(central.len()).unwrap().to_le_bytes());
        out.extend(start.to_le_bytes());
        out.extend([0, 0]);
        out
    }

    /// Answers like the vendors: Codex's one-colour mark comes inside a zipped pack.
    fn vendors(calls: &RefCell<Vec<String>>) -> impl Fn(&str, &Path) -> Result<(), String> {
        move |url: &str, out: &Path| {
            calls.borrow_mut().push(url.to_owned());
            let body = if url.ends_with(".zip") {
                zip(&[
                    ("readme.txt", b"hello"),
                    ("OpenAI-logos(new)/SVGs/OpenAI-black-monoblossom.svg", SVG),
                ])
            } else if url.ends_with(".svg") {
                SVG.to_vec()
            } else {
                PNG.to_vec()
            };
            std::fs::write(out, body).map_err(|error| error.to_string())
        }
    }

    #[test]
    fn mono_where_published_and_fetched_once() {
        let dir = folder("mono");
        let calls = RefCell::new(Vec::new());
        let fetch = vendors(&calls);
        let marks = fetch_missing_with(&dir, Style::Mono, &fetch);
        assert_eq!(
            marks.kept.get(&Agent::Claude),
            Some(&Kept::Color(dir.join("claude.png"))),
            "Anthropic publishes no one-colour spark"
        );
        assert_eq!(
            marks.kept.get(&Agent::Codex),
            Some(&Kept::Mono(dir.join("codex-mono.svg")))
        );
        assert_eq!(std::fs::read(dir.join("codex-mono.svg")).unwrap(), SVG);
        assert_eq!(
            marks.kept.get(&Agent::Agy),
            Some(&Kept::Mono(dir.join("agy-mono.svg")))
        );
        assert_eq!(calls.borrow().len(), 3);

        let again = fetch_missing_with(&dir, Style::Mono, &fetch);
        assert_eq!(calls.borrow().len(), 3, "nothing fetched twice");
        assert_eq!(again, marks);
        let left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".part") || name.ends_with(".download"))
            .collect();
        assert!(left.is_empty(), "{left:?}");

        let color = fetch_missing_with(&dir, Style::Color, &fetch);
        assert_eq!(
            color.kept.get(&Agent::Codex),
            Some(&Kept::Color(dir.join("codex.png")))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn failures_and_wrong_files_leave_the_drawn_mark() {
        let dir = folder("fail");
        let fetch = |url: &str, out: &Path| {
            if url.contains("openai") {
                return Err("offline".to_owned());
            }
            let body: &[u8] = if url.contains("claude") {
                b"<html>moved</html>"
            } else {
                b""
            };
            std::fs::write(out, body).map_err(|error| error.to_string())
        };
        let marks = fetch_missing_with(&dir, Style::Mono, &fetch);
        assert_eq!(marks, Marks::default());
        let left: Vec<_> = std::fs::read_dir(&dir).unwrap().collect();
        assert!(left.is_empty(), "no partial files are left: {left:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zip_entries_are_found_or_refused() {
        let archive = zip(&[("a.txt", b"first"), ("b/c.svg", SVG)]);
        assert_eq!(zip_entry(&archive, "b/c.svg").unwrap(), SVG);
        assert!(
            zip_entry(&archive, "missing.svg")
                .unwrap_err()
                .contains("not in")
        );
        assert!(zip_entry(b"PK not really", "a.txt").is_err());
        assert!(zip_entry(&archive[..archive.len() - 5], "a.txt").is_err());
    }

    #[test]
    fn one_colour_marks_must_be_svg() {
        let dir = folder("check");
        std::fs::create_dir_all(&dir).unwrap();
        let png = dir.join("x.png");
        std::fs::write(&png, PNG).unwrap();
        assert!(check(&png, false).is_ok());
        assert!(check(&png, true).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
