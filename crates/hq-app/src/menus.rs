//! The macOS menu bar (add-release-installer D9). Each item dispatches an action that
//! runs what its shortcut already does, and only shortcuts from `shortcuts.rs` are
//! bound, so menus and keys cannot disagree and no agent key is taken. Linux has no app
//! menu bar; its shortcuts stay key handlers only.

use gpui::{App, KeyBinding, Menu, MenuItem, OsAction, actions};

use crate::shortcuts::Shortcut;

actions!(
    headquarter,
    [
        About,
        CheckForUpdates,
        RemoteControl,
        ThemeSwitcher,
        Hide,
        HideOthers,
        Quit,
        Copy,
        Paste,
        Minimize,
        NewSession,
        OpenSessions,
        ToggleShell,
    ]
);

/// The key each bound action answers to, as `shortcuts::shortcut` reads it.
const BOUND: [(&str, Shortcut); 7] = [
    ("g", Shortcut::ThemeSwitcher),
    ("h", Shortcut::Hide),
    ("q", Shortcut::Quit),
    ("m", Shortcut::Minimize),
    ("n", Shortcut::NewSession),
    ("o", Shortcut::OpenSessions),
    ("t", Shortcut::ToggleShell),
];

/// Sets the menus and their key bindings, and handles the app-wide actions; the
/// workspace handles the rest. Does nothing off macOS.
pub fn install(cx: &mut App) {
    if !cfg!(target_os = "macos") {
        return;
    }
    cx.bind_keys(
        BOUND
            .iter()
            .filter_map(|&(key, shortcut)| binding(key, shortcut)),
    );
    // The terminal's own copy and paste keys; text fields keep their own bindings.
    cx.bind_keys([
        KeyBinding::new("cmd-c", Copy, Some("Terminal")),
        KeyBinding::new("cmd-v", Paste, Some("Terminal")),
    ]);
    cx.on_action(|_: &About, _| show_about());
    cx.on_action(|_: &Hide, cx| cx.hide());
    cx.on_action(|_: &HideOthers, cx| cx.hide_other_apps());
    cx.on_action(|_: &Quit, cx| cx.quit());
    cx.set_menus(vec![
        Menu::new("Headquarter").items([
            MenuItem::action("About Headquarter", About),
            MenuItem::action("Check for Updates…", CheckForUpdates),
            MenuItem::action("Remote Control…", RemoteControl),
            MenuItem::separator(),
            MenuItem::action("Themes…", ThemeSwitcher),
            MenuItem::separator(),
            MenuItem::action("Hide Headquarter", Hide),
            MenuItem::action("Hide Others", HideOthers),
            MenuItem::separator(),
            MenuItem::action("Quit Headquarter", Quit),
        ]),
        Menu::new("Edit").items([
            MenuItem::os_action("Copy", Copy, OsAction::Copy),
            MenuItem::os_action("Paste", Paste, OsAction::Paste),
        ]),
        Menu::new("Window").items([
            MenuItem::action("Minimize", Minimize),
            MenuItem::separator(),
            MenuItem::action("New Session", NewSession),
            MenuItem::action("Open Sessions", OpenSessions),
            MenuItem::action("Shell", ToggleShell),
        ]),
    ]);
}

/// The binding for a menu shortcut; `None` for shortcuts no menu shows.
fn binding(key: &str, shortcut: Shortcut) -> Option<KeyBinding> {
    let keystroke = format!("cmd-{key}");
    let binding = match shortcut {
        Shortcut::ThemeSwitcher => KeyBinding::new(&keystroke, ThemeSwitcher, None),
        Shortcut::Hide => KeyBinding::new(&keystroke, Hide, None),
        Shortcut::Quit => KeyBinding::new(&keystroke, Quit, None),
        Shortcut::Minimize => KeyBinding::new(&keystroke, Minimize, None),
        Shortcut::NewSession => KeyBinding::new(&keystroke, NewSession, None),
        Shortcut::OpenSessions => KeyBinding::new(&keystroke, OpenSessions, None),
        Shortcut::ToggleShell => KeyBinding::new(&keystroke, ToggleShell, None),
        _ => return None,
    };
    Some(binding)
}

/// The standard About panel: the bundle's name, version and icon.
#[cfg(target_os = "macos")]
fn show_about() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    if let Some(main_thread) = MainThreadMarker::new() {
        let app = NSApplication::sharedApplication(main_thread);
        app.orderFrontStandardAboutPanel(None);
    }
}

#[cfg(not(target_os = "macos"))]
fn show_about() {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shortcuts::{Keys, shortcut};

    #[test]
    fn every_bound_key_is_an_app_shortcut_on_cmd() {
        let cmd = Keys {
            platform: true,
            ..Keys::default()
        };
        for (key, expected) in BOUND {
            assert_eq!(shortcut(key, cmd, true), Some(expected), "cmd-{key}");
            assert!(binding(key, expected).is_some(), "cmd-{key} has an action");
        }
    }

    #[test]
    fn no_bound_key_takes_a_plain_ctrl_key_from_an_agent() {
        let ctrl = Keys {
            control: true,
            ..Keys::default()
        };
        for (key, _) in BOUND {
            assert_eq!(shortcut(key, ctrl, true), None, "ctrl-{key}");
        }
    }
}
