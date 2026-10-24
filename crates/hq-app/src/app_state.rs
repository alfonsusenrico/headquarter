//! App-only state kept between runs in `<data folder>/app-state.json`: the sidebar's
//! width and whether it is the icon rail (`openspec/changes/add-sidebar-rail/design.md`,
//! D4), the theme the owner chose (add-theme-switcher D4), and the window's place and
//! the session it showed (add-session-restore D3). Other keys in the file are kept as
//! they are, so later state can join it.

use std::path::{Path, PathBuf};

use serde_json::{Value, json};

pub const DEFAULT_WIDTH: f32 = 280.0;
/// The narrowest full sidebar. Narrower, the rail is drawn, and a drag released there
/// settles on the rail.
pub const MIN_WIDTH: f32 = 180.0;
pub const MAX_WIDTH: f32 = 480.0;
pub const RAIL_WIDTH: f32 = 56.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SidebarState {
    /// The full sidebar's width, kept while the rail is shown.
    pub width: f32,
    pub rail: bool,
}

impl Default for SidebarState {
    fn default() -> SidebarState {
        SidebarState {
            width: DEFAULT_WIDTH,
            rail: false,
        }
    }
}

impl SidebarState {
    /// The state with the divider dragged to `x` pixels from the window's left edge.
    pub fn dragged_to(self, x: f32) -> SidebarState {
        if x < MIN_WIDTH {
            SidebarState { rail: true, ..self }
        } else {
            SidebarState {
                width: x.clamp(MIN_WIDTH, MAX_WIDTH),
                rail: false,
            }
        }
    }

    pub fn toggled(self) -> SidebarState {
        SidebarState {
            rail: !self.rail,
            ..self
        }
    }

    /// The width the sidebar takes on screen.
    pub fn shown_width(self) -> f32 {
        if self.rail { RAIL_WIDTH } else { self.width }
    }
}

/// The sidebar's width while the divider is at `x`: it follows the pointer all the way
/// down to the rail's width.
pub fn live_width(x: f32) -> f32 {
    x.clamp(RAIL_WIDTH, MAX_WIDTH)
}

/// Whether a sidebar this wide is drawn as the rail.
pub fn is_rail(width: f32) -> bool {
    width < MIN_WIDTH
}

pub fn path() -> Option<PathBuf> {
    hq_signals::paths::data_dir().map(|data| data.join("app-state.json"))
}

/// The kept sidebar state; a missing or unreadable file, or odd values, give the
/// defaults.
pub fn load_sidebar(path: &Path) -> SidebarState {
    let Some(sidebar) = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|state| state.get("sidebar").cloned())
    else {
        return SidebarState::default();
    };
    let defaults = SidebarState::default();
    let width = sidebar
        .get("width")
        .and_then(Value::as_f64)
        .filter(|width| width.is_finite())
        .map_or(defaults.width, |width| {
            (width as f32).clamp(MIN_WIDTH, MAX_WIDTH)
        });
    let rail = sidebar
        .get("rail")
        .and_then(Value::as_bool)
        .unwrap_or(defaults.rail);
    SidebarState { width, rail }
}

/// Writes the sidebar state, keeping the file's other keys.
pub fn save_sidebar(path: &Path, sidebar: SidebarState) -> std::io::Result<()> {
    save_entry(
        path,
        "sidebar",
        json!({ "width": sidebar.width.round(), "rail": sidebar.rail }),
    )
}

/// The kept theme choice's name, if one is kept.
pub fn load_theme(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let state = serde_json::from_slice::<Value>(&bytes).ok()?;
    state.get("theme")?.as_str().map(str::to_owned)
}

/// Writes the theme choice's name, keeping the file's other keys.
pub fn save_theme(path: &Path, name: &str) -> std::io::Result<()> {
    save_entry(path, "theme", json!(name))
}

/// Where the window was, in screen points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowPlace {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// The smallest window worth restoring; anything smaller is treated as odd.
const MIN_WINDOW: (f32, f32) = (400.0, 300.0);

/// The kept window place, if the file has a sensible one.
pub fn load_window(path: &Path) -> Option<WindowPlace> {
    let state = read(path)?;
    let window = state.get("window")?;
    let number = |key: &str| window.get(key)?.as_f64().map(|value| value as f32);
    let place = WindowPlace {
        x: number("x")?,
        y: number("y")?,
        width: number("width")?,
        height: number("height")?,
    };
    (place.width >= MIN_WINDOW.0 && place.height >= MIN_WINDOW.1).then_some(place)
}

pub fn save_window(path: &Path, place: WindowPlace) -> std::io::Result<()> {
    save_entry(
        path,
        "window",
        json!({
            "x": place.x.round(),
            "y": place.y.round(),
            "width": place.width.round(),
            "height": place.height.round(),
        }),
    )
}

/// Whether the window would be usable on one of `displays`: its top-left corner region
/// lies on that display and it is no larger than it.
pub fn fits(place: WindowPlace, displays: &[WindowPlace]) -> bool {
    const GRIP: f32 = 80.0;
    displays.iter().any(|display| {
        place.x >= display.x
            && place.y >= display.y
            && place.x + GRIP <= display.x + display.width
            && place.y + GRIP <= display.y + display.height
            && place.width <= display.width
            && place.height <= display.height
    })
}

/// The id of the session the window showed, if one is kept.
pub fn load_shown(path: &Path) -> Option<String> {
    read(path)?
        .get("shown_session")?
        .as_str()
        .map(str::to_owned)
}

pub fn save_shown(path: &Path, id: &str) -> std::io::Result<()> {
    save_entry(path, "shown_session", json!(id))
}

/// The sessions whose shell panel is open (add-session-shell-panel).
pub fn load_shell_panels(path: &Path) -> Vec<String> {
    read(path)
        .and_then(|state| state.get("shell_panels").cloned())
        .and_then(|value| serde_json::from_value::<Vec<String>>(value).ok())
        .unwrap_or_default()
}

pub fn save_shell_panels(path: &Path, sessions: &[String]) -> std::io::Result<()> {
    save_entry(path, "shell_panels", json!(sessions))
}

/// The shell panel's share of the pane's height, a third until the owner drags it.
pub const SHELL_SHARE: f32 = 1.0 / 3.0;

pub fn load_shell_share(path: &Path) -> f32 {
    read(path)
        .and_then(|state| state.get("shell_share")?.as_f64())
        .map(|share| share as f32)
        .filter(|share| (0.1..=0.9).contains(share))
        .unwrap_or(SHELL_SHARE)
}

pub fn save_shell_share(path: &Path, share: f32) -> std::io::Result<()> {
    save_entry(path, "shell_share", json!(share))
}

fn read(path: &Path) -> Option<Value> {
    serde_json::from_slice::<Value>(&std::fs::read(path).ok()?).ok()
}

/// Sets one key of the file and keeps the rest. The write goes to a temporary file
/// first, so a crash never leaves half a file.
fn save_entry(path: &Path, key: &str, value: Value) -> std::io::Result<()> {
    let mut state = std::fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}));
    state[key] = value;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let partial = path.with_extension("json.part");
    std::fs::write(&partial, serde_json::to_vec_pretty(&state)?)?;
    std::fs::rename(&partial, path)
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// The state file in a fresh folder; the folder goes when the test ends.
    struct StateFile {
        _dir: TempPath,
        file: PathBuf,
    }

    impl std::ops::Deref for StateFile {
        type Target = Path;

        fn deref(&self) -> &Path {
            &self.file
        }
    }

    impl AsRef<Path> for StateFile {
        fn as_ref(&self) -> &Path {
            &self.file
        }
    }

    fn file(name: &str) -> StateFile {
        let dir = std::env::temp_dir().join(format!("hq-state-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("app-state.json");
        StateFile {
            _dir: TempPath(dir),
            file,
        }
    }

    #[test]
    fn dragging_clamps_and_snaps() {
        let start = SidebarState::default();
        assert_eq!(start.dragged_to(340.0).width, 340.0);
        assert_eq!(start.dragged_to(900.0).width, MAX_WIDTH);
        assert_eq!(start.dragged_to(190.0).width, 190.0);
        let rail = start.dragged_to(150.0);
        assert!(rail.rail);
        assert_eq!(
            rail.width, DEFAULT_WIDTH,
            "the full width is kept for later"
        );
        assert_eq!(rail.shown_width(), RAIL_WIDTH);
        let back = rail.dragged_to(260.0);
        assert_eq!((back.rail, back.width), (false, 260.0));
        assert_eq!(rail.toggled(), SidebarState::default());
    }

    #[test]
    fn the_live_width_follows_the_pointer_down_to_the_rail() {
        assert_eq!(live_width(200.0), 200.0);
        assert_eq!(live_width(120.0), 120.0);
        assert_eq!(live_width(10.0), RAIL_WIDTH);
        assert_eq!(live_width(900.0), MAX_WIDTH);
        assert!(is_rail(120.0) && is_rail(RAIL_WIDTH));
        assert!(!is_rail(MIN_WIDTH));
    }

    #[test]
    fn saved_and_read_back_keeping_other_keys() {
        let path = file("save");
        std::fs::write(&path, r#"{"window":{"x":5}}"#).unwrap();
        let state = SidebarState {
            width: 333.4,
            rail: true,
        };
        save_sidebar(&path, state).unwrap();
        assert_eq!(
            load_sidebar(&path),
            SidebarState {
                width: 333.0,
                rail: true
            }
        );
        let kept: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(kept["window"]["x"], 5);
    }

    #[test]
    fn the_theme_is_kept_beside_the_sidebar() {
        let path = file("theme");
        assert_eq!(load_theme(&path), None, "no file, no choice");
        save_sidebar(&path, SidebarState::default()).unwrap();
        save_theme(&path, "dark").unwrap();
        assert_eq!(load_theme(&path).as_deref(), Some("dark"));
        assert_eq!(load_sidebar(&path), SidebarState::default());
        std::fs::write(&path, r#"{"theme":7}"#).unwrap();
        assert_eq!(load_theme(&path), None, "an odd value is no choice");
    }

    #[test]
    fn shell_panels_and_their_share_are_kept_with_the_rest() {
        let path = file("shell");
        assert!(load_shell_panels(&path).is_empty());
        assert_eq!(load_shell_share(&path), SHELL_SHARE, "a third by default");
        save_shown(&path, "abc123").unwrap();
        save_shell_panels(&path, &["abc123".to_owned(), "def456".to_owned()]).unwrap();
        save_shell_share(&path, 0.5).unwrap();
        assert_eq!(load_shell_panels(&path), ["abc123", "def456"]);
        assert!((load_shell_share(&path) - 0.5).abs() < f32::EPSILON);
        assert_eq!(
            load_shown(&path).as_deref(),
            Some("abc123"),
            "the rest is kept"
        );
        save_shell_share(&path, 7.0).unwrap();
        assert_eq!(
            load_shell_share(&path),
            SHELL_SHARE,
            "an impossible share is ignored"
        );
    }

    #[test]
    fn the_window_and_shown_session_are_kept_with_the_rest() {
        let path = file("window");
        save_sidebar(&path, SidebarState::default()).unwrap();
        save_theme(&path, "dark").unwrap();
        let place = WindowPlace {
            x: 960.4,
            y: 25.0,
            width: 960.0,
            height: 1055.0,
        };
        save_window(&path, place).unwrap();
        save_shown(&path, "abc123").unwrap();
        assert_eq!(load_window(&path), Some(WindowPlace { x: 960.0, ..place }));
        assert_eq!(load_shown(&path).as_deref(), Some("abc123"));
        assert_eq!(load_theme(&path).as_deref(), Some("dark"));
        assert_eq!(load_sidebar(&path), SidebarState::default());
        std::fs::write(&path, r#"{"window":{"x":0,"y":0,"width":10,"height":10}}"#).unwrap();
        assert_eq!(load_window(&path), None, "a tiny window is odd");
        std::fs::write(&path, r#"{"window":"left half"}"#).unwrap();
        assert_eq!(load_window(&path), None);
    }

    #[test]
    fn a_window_fits_only_on_a_connected_display() {
        let main = WindowPlace {
            x: 0.0,
            y: 0.0,
            width: 1728.0,
            height: 1117.0,
        };
        let right_half = WindowPlace {
            x: 864.0,
            y: 25.0,
            width: 864.0,
            height: 1092.0,
        };
        assert!(fits(right_half, &[main]));
        let on_external = WindowPlace {
            x: 2000.0,
            ..right_half
        };
        assert!(!fits(on_external, &[main]), "the external display is gone");
        let external = WindowPlace {
            x: 1728.0,
            y: 0.0,
            width: 2560.0,
            height: 1440.0,
        };
        assert!(fits(on_external, &[main, external]));
        let too_big = WindowPlace {
            width: 3000.0,
            ..right_half
        };
        assert!(!fits(too_big, &[main]));
    }

    #[test]
    fn missing_or_odd_files_give_defaults() {
        let path = file("odd");
        assert_eq!(load_sidebar(&path), SidebarState::default());
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(load_sidebar(&path), SidebarState::default());
        std::fs::write(&path, r#"{"sidebar":{"width":9000,"rail":"yes"}}"#).unwrap();
        assert_eq!(
            load_sidebar(&path),
            SidebarState {
                width: MAX_WIDTH,
                rail: false
            }
        );
        std::fs::write(&path, "[1]").unwrap();
        save_sidebar(&path, SidebarState::default()).unwrap();
        assert_eq!(load_sidebar(&path), SidebarState::default());
    }
}
