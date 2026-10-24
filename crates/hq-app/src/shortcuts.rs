//! App shortcuts (design D8). On macOS they use Cmd, which agents never receive; on
//! Linux they use Ctrl-Shift, so plain Ctrl keys such as Ctrl-N and Ctrl-J reach the
//! agents unchanged.

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

/// The app shortcut for `key` with `keys` held, if it is one.
pub fn shortcut(key: &str, keys: Keys, mac: bool) -> Option<Shortcut> {
    if !is_command(keys, mac) {
        return None;
    }
    match key {
        "1" | "!" => Some(Shortcut::FocusSidebar),
        "2" | "@" => Some(Shortcut::FocusTerminal),
        "n" => Some(Shortcut::NewSession),
        "o" => Some(Shortcut::OpenSessions),
        "j" => Some(Shortcut::NextNeedingYou),
        "b" => Some(Shortcut::ToggleRail),
        "g" => Some(Shortcut::ThemeSwitcher),
        "q" => Some(Shortcut::Quit),
        "m" => Some(Shortcut::Minimize),
        "h" if mac => Some(Shortcut::Hide),
        "t" => Some(Shortcut::ToggleShell),
        _ => None,
    }
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
