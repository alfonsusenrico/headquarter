//! App shortcuts (design D8). On macOS they use Cmd, which agents never receive; on
//! Linux they use Ctrl-Shift, so plain Ctrl keys such as Ctrl-N and Ctrl-J reach the
//! agents unchanged. `SHORTCUTS` is the one table the key handling, the menus and the
//! Keyboard Shortcuts window read (add-shortcuts-window D1).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shortcut {
    FocusSidebar,
    FocusTerminal,
    NewSession,
    OpenSessions,
    NextNeedingYou,
    /// Switches the sidebar between full and the icon rail.
    ToggleRail,
    ThemeSwitcher,
    Quit,
    Minimize,
    /// Hides the app; macOS only.
    Hide,
    /// Opens or closes the shown session's shell panel (add-session-shell-panel).
    ToggleShell,
    /// Opens or closes the Keyboard Shortcuts window.
    KeyboardShortcuts,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Keys {
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    /// Cmd on macOS, Super elsewhere.
    pub platform: bool,
}

/// Whether `keys` hold the app's command modifier on this kind of system.
pub fn is_command(keys: Keys, mac: bool) -> bool {
    if mac {
        keys.platform && !keys.control && !keys.alt
    } else {
        keys.control && keys.shift && !keys.alt && !keys.platform
    }
}

/// Where the Keyboard Shortcuts window lists a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Area {
    Sessions,
    Window,
    Terminal,
    Sidebar,
}

impl Area {
    pub const ALL: [Area; 4] = [Area::Sessions, Area::Window, Area::Terminal, Area::Sidebar];

    pub fn name(self) -> &'static str {
        match self {
            Area::Sessions => "Sessions",
            Area::Window => "Window",
            Area::Terminal => "Terminal",
            Area::Sidebar => "Sidebar",
        }
    }
}

/// An app shortcut: the key pressed with the command modifier, and how the window lists it.
pub struct Entry {
    pub shortcut: Shortcut,
    /// The key as GPUI names it.
    pub key: &'static str,
    /// The same key as many layouts report it with Shift held, which Linux's Ctrl-Shift needs.
    pub shifted: Option<&'static str>,
    pub mac_only: bool,
    pub area: Area,
    pub name: &'static str,
    pub what: &'static str,
}

const fn entry(
    shortcut: Shortcut,
    key: &'static str,
    area: Area,
    name: &'static str,
    what: &'static str,
) -> Entry {
    Entry {
        shortcut,
        key,
        shifted: None,
        mac_only: false,
        area,
        name,
        what,
    }
}

/// Every app shortcut, in the order the window lists them.
pub const SHORTCUTS: [Entry; 12] = [
    entry(
        Shortcut::NewSession,
        "n",
        Area::Sessions,
        "New Session",
        "Start an agent in a folder.",
    ),
    entry(
        Shortcut::OpenSessions,
        "o",
        Area::Sessions,
        "Open Sessions",
        "Find a session, or add past conversations to the sidebar.",
    ),
    entry(
        Shortcut::NextNeedingYou,
        "j",
        Area::Sessions,
        "Next Needing You",
        "Jump to the next session that needs you.",
    ),
    Entry {
        shifted: Some("!"),
        ..entry(
            Shortcut::FocusSidebar,
            "1",
            Area::Window,
            "Focus Sidebar",
            "Move the keyboard to the sidebar.",
        )
    },
    Entry {
        shifted: Some("@"),
        ..entry(
            Shortcut::FocusTerminal,
            "2",
            Area::Window,
            "Focus Terminal",
            "Move the keyboard to the session's terminal.",
        )
    },
    entry(
        Shortcut::ToggleRail,
        "b",
        Area::Window,
        "Sidebar Rail",
        "Switch the sidebar between full and icons only.",
    ),
    entry(
        Shortcut::ThemeSwitcher,
        "g",
        Area::Window,
        "Themes",
        "Pick a light, dark or custom theme.",
    ),
    Entry {
        shifted: Some("?"),
        ..entry(
            Shortcut::KeyboardShortcuts,
            "/",
            Area::Window,
            "Keyboard Shortcuts",
            "This window.",
        )
    },
    entry(
        Shortcut::Minimize,
        "m",
        Area::Window,
        "Minimize",
        "Minimize the window.",
    ),
    Entry {
        mac_only: true,
        ..entry(
            Shortcut::Hide,
            "h",
            Area::Window,
            "Hide Headquarter",
            "Hide the app.",
        )
    },
    entry(
        Shortcut::Quit,
        "q",
        Area::Window,
        "Quit",
        "Quit Headquarter, or keep its sessions running.",
    ),
    entry(
        Shortcut::ToggleShell,
        "t",
        Area::Terminal,
        "Shell Panel",
        "Open or close the session's shell panel.",
    ),
];

/// The app shortcut for `key` with `keys` held, if it is one.
pub fn shortcut(key: &str, keys: Keys, mac: bool) -> Option<Shortcut> {
    if !is_command(keys, mac) {
        return None;
    }
    SHORTCUTS
        .iter()
        .find(|entry| (entry.key == key || entry.shifted == Some(key)) && (mac || !entry.mac_only))
        .map(|entry| entry.shortcut)
}

/// How a listed key is pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Press {
    /// With the command modifier: ⌘ on macOS, Ctrl+Shift on Linux.
    Command(&'static str),
    /// Any one of these keys alone.
    Alone(&'static [&'static str]),
    /// Any one of these keys with Alt and Shift.
    AltShift(&'static [&'static str]),
}

impl Press {
    /// The key caps to draw: one group for each key that does it.
    pub fn caps(self, mac: bool) -> Vec<Vec<String>> {
        let with = |modifiers: &[&str], keys: &[&str]| {
            keys.iter()
                .map(|key| {
                    modifiers
                        .iter()
                        .chain(std::iter::once(key))
                        .map(|cap| (*cap).to_owned())
                        .collect()
                })
                .collect()
        };
        match self {
            Press::Command(key) => {
                let key = key.to_uppercase();
                let caps: Vec<String> = if mac {
                    vec!["⌘".to_owned(), key]
                } else {
                    vec!["Ctrl".to_owned(), "Shift".to_owned(), key]
                };
                vec![caps]
            }
            Press::Alone(keys) => with(&[], keys),
            Press::AltShift(keys) if mac => with(&["⌥", "⇧"], keys),
            Press::AltShift(keys) => with(&["Alt", "Shift"], keys),
        }
    }

    /// The caps as the window draws them: modifiers once, before the first key, so
    /// Alt-Shift-arrows reads "⌥ ⇧ ↑ / ↓" rather than repeating them.
    pub fn shown(self, mac: bool) -> Vec<Vec<String>> {
        let mut groups = self.caps(mac);
        for group in groups.iter_mut().skip(1) {
            group.drain(..group.len() - 1);
        }
        groups
    }
}

/// One row of the Keyboard Shortcuts window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listing {
    pub area: Area,
    pub name: &'static str,
    pub what: &'static str,
    pub press: Press,
}

/// Keys the window lists that are not app shortcuts: the terminal's copy and paste
/// (`terminal::view`) and the sidebar's own keys (`Workspace::sidebar_key_down`).
const OTHER_KEYS: [Listing; 7] = [
    Listing {
        area: Area::Terminal,
        name: "Copy",
        what: "Copy the selected text.",
        press: Press::Command("c"),
    },
    Listing {
        area: Area::Terminal,
        name: "Paste",
        what: "Paste into the terminal.",
        press: Press::Command("v"),
    },
    Listing {
        area: Area::Sidebar,
        name: "Choose a session",
        what: "While the sidebar has the keyboard.",
        press: Press::Alone(&["↑", "↓"]),
    },
    Listing {
        area: Area::Sidebar,
        name: "Open it",
        what: "Show the chosen session in the pane.",
        press: Press::Alone(&["Enter"]),
    },
    Listing {
        area: Area::Sidebar,
        name: "Collapse or expand its group",
        what: "On a session in a group.",
        press: Press::Alone(&["←", "→"]),
    },
    Listing {
        area: Area::Sidebar,
        name: "Move it to another group",
        what: "Up or down one group.",
        press: Press::AltShift(&["↑", "↓"]),
    },
    Listing {
        area: Area::Sidebar,
        name: "Cancel",
        what: "Back out of a close or delete prompt.",
        press: Press::Alone(&["Esc"]),
    },
];

/// Every row of the Keyboard Shortcuts window on this kind of system, by area.
pub fn listings(mac: bool) -> Vec<Listing> {
    let shortcuts = SHORTCUTS
        .iter()
        .filter(|entry| mac || !entry.mac_only)
        .map(|entry| Listing {
            area: entry.area,
            name: entry.name,
            what: entry.what,
            press: Press::Command(entry.key),
        });
    let mut listings: Vec<Listing> = shortcuts.chain(OTHER_KEYS).collect();
    listings.sort_by_key(|listing| Area::ALL.iter().position(|area| *area == listing.area));
    listings
}

/// Whether a row matches what the owner typed: its name, explanation, area or keys, with
/// spaces and `+` ignored so "⌘t", "cmd+t" and "ctrl shift t" all find Shell Panel.
pub fn matches(listing: &Listing, query: &str, mac: bool) -> bool {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    let mut keys: Vec<String> = listing
        .press
        .caps(mac)
        .iter()
        .map(|caps| caps.join("+"))
        .collect();
    if let (Press::Command(key), true) = (listing.press, mac) {
        keys.push(format!("cmd+{key}"));
    }
    let text = format!(
        "{} {} {} {}",
        listing.name,
        listing.what,
        listing.area.name(),
        keys.join(" ")
    )
    .to_lowercase();
    let squash = |text: &str| -> String {
        text.chars()
            .filter(|c| !c.is_whitespace() && *c != '+')
            .collect()
    };
    text.contains(&query) || squash(&text).contains(&squash(&query))
}

pub const MAC: bool = cfg!(target_os = "macos");

#[cfg(test)]
mod tests {
    use super::*;

    const CTRL: Keys = Keys {
        control: true,
        alt: false,
        shift: false,
        platform: false,
    };

    const CMD: Keys = Keys {
        control: false,
        alt: false,
        shift: false,
        platform: true,
    };

    const CTRL_SHIFT: Keys = Keys {
        control: true,
        alt: false,
        shift: true,
        platform: false,
    };

    /// Every shortcut; the match fails to compile when one is added without a row here.
    const ALL: [Shortcut; 12] = [
        Shortcut::FocusSidebar,
        Shortcut::FocusTerminal,
        Shortcut::NewSession,
        Shortcut::OpenSessions,
        Shortcut::NextNeedingYou,
        Shortcut::ToggleRail,
        Shortcut::ThemeSwitcher,
        Shortcut::Quit,
        Shortcut::Minimize,
        Shortcut::Hide,
        Shortcut::ToggleShell,
        Shortcut::KeyboardShortcuts,
    ];

    fn _every_shortcut_is_in_all(shortcut: Shortcut) {
        match shortcut {
            Shortcut::FocusSidebar
            | Shortcut::FocusTerminal
            | Shortcut::NewSession
            | Shortcut::OpenSessions
            | Shortcut::NextNeedingYou
            | Shortcut::ToggleRail
            | Shortcut::ThemeSwitcher
            | Shortcut::Quit
            | Shortcut::Minimize
            | Shortcut::Hide
            | Shortcut::ToggleShell
            | Shortcut::KeyboardShortcuts => {}
        }
    }

    #[test]
    fn every_shortcut_is_listed_once_and_no_two_share_a_key() {
        for shortcut in ALL {
            let count = SHORTCUTS
                .iter()
                .filter(|entry| entry.shortcut == shortcut)
                .count();
            assert_eq!(count, 1, "{shortcut:?}");
        }
        for (index, entry) in SHORTCUTS.iter().enumerate() {
            for other in &SHORTCUTS[index + 1..] {
                let keys = [Some(entry.key), entry.shifted];
                let shared = keys
                    .iter()
                    .flatten()
                    .any(|key| *key == other.key || Some(*key) == other.shifted);
                assert!(!shared, "{} and {} share a key", entry.name, other.name);
            }
        }
    }

    #[test]
    fn the_window_lists_every_key_by_area_for_its_system() {
        let mac = listings(true);
        let linux = listings(false);
        assert_eq!(mac.len(), SHORTCUTS.len() + OTHER_KEYS.len());
        assert!(mac.iter().any(|row| row.name == "Hide Headquarter"));
        assert!(
            !linux.iter().any(|row| row.name == "Hide Headquarter"),
            "Linux has no hiding an app"
        );
        let areas: Vec<Area> = mac.iter().map(|row| row.area).collect();
        let mut sorted = areas.clone();
        sorted.dedup();
        assert_eq!(sorted, Area::ALL, "rows come area by area");
        let shell = mac.iter().find(|row| row.name == "Shell Panel").unwrap();
        assert_eq!(shell.press.caps(true), vec![vec!["⌘", "T"]]);
        assert_eq!(shell.press.caps(false), vec![vec!["Ctrl", "Shift", "T"]]);
        let move_row = mac
            .iter()
            .find(|row| row.name == "Move it to another group")
            .unwrap();
        assert_eq!(
            move_row.press.caps(false),
            vec![vec!["Alt", "Shift", "↑"], vec!["Alt", "Shift", "↓"]]
        );
        assert_eq!(
            move_row.press.shown(true),
            vec![vec!["⌥", "⇧", "↑"], vec!["↓"]]
        );
        assert_eq!(shell.press.shown(false), shell.press.caps(false));
    }

    #[test]
    fn typing_filters_by_name_explanation_or_keys() {
        let names = |query: &str, mac: bool| -> Vec<&str> {
            listings(mac)
                .into_iter()
                .filter(|row| matches(row, query, mac))
                .map(|row| row.name)
                .collect()
        };
        assert_eq!(names("shell", true), ["Shell Panel"]);
        assert_eq!(names("⌘t", true), ["Shell Panel"]);
        assert_eq!(names("cmd+t", true), ["Shell Panel"]);
        assert_eq!(names("ctrl shift t", false), ["Shell Panel"]);
        assert_eq!(names("FOCUS", true), ["Focus Sidebar", "Focus Terminal"]);
        assert_eq!(names("", true).len(), listings(true).len());
        assert!(names("zzz", true).is_empty());
    }

    #[test]
    fn the_window_opens_on_slash_or_question_mark() {
        assert_eq!(shortcut("/", CMD, true), Some(Shortcut::KeyboardShortcuts));
        assert_eq!(
            shortcut("/", CTRL_SHIFT, false),
            Some(Shortcut::KeyboardShortcuts)
        );
        assert_eq!(
            shortcut("?", CTRL_SHIFT, false),
            Some(Shortcut::KeyboardShortcuts),
            "Shift turns / into ? on many layouts"
        );
        assert_eq!(
            shortcut("/", CTRL, false),
            None,
            "Ctrl-/ stays with the agent"
        );
    }

    #[test]
    fn the_shell_panel_takes_cmd_t_or_ctrl_shift_t_only() {
        let cmd = Keys {
            platform: true,
            ..Keys::default()
        };
        let ctrl_shift = Keys {
            shift: true,
            ..CTRL
        };
        assert_eq!(shortcut("t", cmd, true), Some(Shortcut::ToggleShell));
        assert_eq!(
            shortcut("t", ctrl_shift, false),
            Some(Shortcut::ToggleShell)
        );
        assert_eq!(
            shortcut("t", CTRL, false),
            None,
            "Ctrl-T stays with the agent"
        );
        assert_eq!(shortcut("t", CTRL, true), None);
    }

    #[test]
    fn mac_uses_cmd() {
        let cmd = Keys {
            platform: true,
            ..Keys::default()
        };
        assert_eq!(shortcut("n", cmd, true), Some(Shortcut::NewSession));
        assert_eq!(shortcut("o", cmd, true), Some(Shortcut::OpenSessions));
        assert_eq!(shortcut("j", cmd, true), Some(Shortcut::NextNeedingYou));
        assert_eq!(shortcut("b", cmd, true), Some(Shortcut::ToggleRail));
        assert_eq!(shortcut("1", cmd, true), Some(Shortcut::FocusSidebar));
        assert_eq!(shortcut("g", cmd, true), Some(Shortcut::ThemeSwitcher));
        assert_eq!(shortcut("q", cmd, true), Some(Shortcut::Quit));
        assert_eq!(shortcut("m", cmd, true), Some(Shortcut::Minimize));
        assert_eq!(shortcut("h", cmd, true), Some(Shortcut::Hide));
        assert_eq!(
            shortcut("n", CTRL, true),
            None,
            "Ctrl-N belongs to the agent"
        );
        assert_eq!(shortcut("x", cmd, true), None);
    }

    #[test]
    fn linux_leaves_plain_ctrl_to_the_agents() {
        assert_eq!(shortcut("n", CTRL, false), None);
        assert_eq!(shortcut("j", CTRL, false), None);
        assert_eq!(
            shortcut("b", CTRL, false),
            None,
            "tmux and readline use Ctrl-B"
        );
        let ctrl_shift = Keys {
            shift: true,
            ..CTRL
        };
        assert_eq!(shortcut("n", ctrl_shift, false), Some(Shortcut::NewSession));
        assert_eq!(
            shortcut("j", ctrl_shift, false),
            Some(Shortcut::NextNeedingYou)
        );
        assert_eq!(shortcut("b", ctrl_shift, false), Some(Shortcut::ToggleRail));
        assert_eq!(
            shortcut("g", CTRL, false),
            None,
            "Claude Code edits the prompt in an editor on Ctrl-G"
        );
        assert_eq!(shortcut("q", CTRL, false), None);
        assert_eq!(shortcut("m", CTRL, false), None);
        assert_eq!(
            shortcut("g", ctrl_shift, false),
            Some(Shortcut::ThemeSwitcher)
        );
        assert_eq!(shortcut("q", ctrl_shift, false), Some(Shortcut::Quit));
        assert_eq!(shortcut("m", ctrl_shift, false), Some(Shortcut::Minimize));
        assert_eq!(
            shortcut("h", ctrl_shift, false),
            None,
            "Linux has no hiding an app"
        );
        assert_eq!(
            shortcut("!", ctrl_shift, false),
            Some(Shortcut::FocusSidebar),
            "Shift turns 1 into ! on many layouts"
        );
    }
}
