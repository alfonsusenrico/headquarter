//! Key presses to the bytes an xterm-compatible terminal sends.
//!
//! Plain printable text is not encoded here: it arrives through the platform's text
//! input (IME) path, which also handles dead keys and composition. This module handles
//! everything else: navigation and function keys, Ctrl and Alt combinations, and the
//! owner's key bindings.

/// A key press, independent of the UI toolkit so it can be tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyPress<'a> {
    /// Key name: a lower-case character, or a name such as "enter", "up", "f5".
    pub key: &'a str,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    /// Cmd on macOS, Super elsewhere: always left to the app.
    pub platform: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub key: String,
    pub control: bool,
    pub alt: bool,
    pub shift: bool,
    pub platform: bool,
    pub chars: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct KeyModes {
    /// DECCKM: cursor keys send SS3 sequences.
    pub app_cursor: bool,
    /// Whether Option/Alt acts as Meta (Escape prefix) instead of composing characters.
    pub alt_is_meta: bool,
}

/// The bytes to send for `press`, or `None` when the key is not the terminal's: app
/// shortcuts, or printable text that arrives through text input.
pub fn encode(press: KeyPress<'_>, modes: KeyModes, bindings: &[Binding]) -> Option<Vec<u8>> {
    if let Some(binding) = bindings.iter().find(|binding| {
        binding.key == press.key
            && binding.control == press.control
            && binding.alt == press.alt
            && binding.shift == press.shift
            && binding.platform == press.platform
    }) {
        return Some(binding.chars.clone());
    }
    if press.platform {
        return None;
    }
    let modifier =
        1 + u8::from(press.shift) + 2 * u8::from(press.alt) + 4 * u8::from(press.control);
    let modified = modifier > 1;
    let csi = |final_byte: char| -> Vec<u8> {
        if modified {
            format!("\x1b[1;{modifier}{final_byte}").into_bytes()
        } else if modes.app_cursor {
            format!("\x1bO{final_byte}").into_bytes()
        } else {
            format!("\x1b[{final_byte}").into_bytes()
        }
    };
    let tilde = |code: u8| -> Vec<u8> {
        if modified {
            format!("\x1b[{code};{modifier}~").into_bytes()
        } else {
            format!("\x1b[{code}~").into_bytes()
        }
    };
    let meta = |bytes: &[u8]| -> Vec<u8> {
        if press.alt && modes.alt_is_meta {
            [b"\x1b".as_slice(), bytes].concat()
        } else {
            bytes.to_vec()
        }
    };

    let bytes = match press.key {
        "up" => csi('A'),
        "down" => csi('B'),
        "right" => csi('C'),
        "left" => csi('D'),
        "home" => csi('H'),
        "end" => csi('F'),
        "insert" => tilde(2),
        "delete" => tilde(3),
        "pageup" => tilde(5),
        "pagedown" => tilde(6),
        "f1" | "f2" | "f3" | "f4" => {
            let final_byte = match press.key {
                "f1" => 'P',
                "f2" => 'Q',
                "f3" => 'R',
                _ => 'S',
            };
            if modified {
                format!("\x1b[1;{modifier}{final_byte}").into_bytes()
            } else {
                format!("\x1bO{final_byte}").into_bytes()
            }
        }
        "f5" => tilde(15),
        "f6" => tilde(17),
        "f7" => tilde(18),
        "f8" => tilde(19),
        "f9" => tilde(20),
        "f10" => tilde(21),
        "f11" => tilde(23),
        "f12" => tilde(24),
        "enter" => meta(b"\r"),
        "escape" => meta(b"\x1b"),
        "tab" if press.shift => b"\x1b[Z".to_vec(),
        "tab" => meta(b"\t"),
        "backspace" if press.control => meta(b"\x08"),
        "backspace" => meta(b"\x7f"),
        "space" if press.control => meta(b"\0"),
        "space" if press.alt && modes.alt_is_meta => meta(b" "),
        key if press.control => meta(&[control_byte(key)?]),
        key if press.alt && modes.alt_is_meta => {
            let mut chars = key.chars();
            let character = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            let character = if press.shift {
                character.to_ascii_uppercase()
            } else {
                character
            };
            meta(character.to_string().as_bytes())
        }
        _ => return None,
    };
    Some(bytes)
}

/// Ctrl+key as a C0 control byte, as xterm sends it.
fn control_byte(key: &str) -> Option<u8> {
    let mut chars = key.chars();
    let character = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    match character {
        'a'..='z' => Some(character as u8 - b'a' + 1),
        '@' | '2' | ' ' => Some(0),
        '[' | '3' => Some(0x1b),
        '\\' | '4' => Some(0x1c),
        ']' | '5' => Some(0x1d),
        '^' | '6' => Some(0x1e),
        '_' | '-' | '7' => Some(0x1f),
        '?' | '8' => Some(0x7f),
        _ => None,
    }
}

/// Wraps pasted text for bracketed paste when the agent asked for it, and strips any
/// end marker inside the text so a paste cannot end the bracket early.
pub fn paste(text: &str, bracketed: bool) -> Vec<u8> {
    if !bracketed {
        return text.replace("\r\n", "\r").replace('\n', "\r").into_bytes();
    }
    let clean = text.replace("\x1b[201~", "");
    [b"\x1b[200~".as_slice(), clean.as_bytes(), b"\x1b[201~"].concat()
}

/// What a drop of files types, as Terminal.app does: each path with shell characters
/// escaped by a backslash, separated by spaces, with a space after the last so the next
/// word does not join it. Agents read an image path typed this way as an attachment.
pub fn dropped_paths(paths: &[std::path::PathBuf]) -> String {
    let mut text = String::new();
    for path in paths {
        for c in path.to_string_lossy().chars() {
            let plain = !c.is_ascii() || c.is_ascii_alphanumeric() || "-_./,:@+=%".contains(c);
            if !plain {
                text.push('\\');
            }
            text.push(c);
        }
        text.push(' ');
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: &str) -> KeyPress<'_> {
        KeyPress {
            key,
            ..KeyPress::default()
        }
    }

    fn send(press: KeyPress<'_>) -> Option<Vec<u8>> {
        encode(press, KeyModes::default(), &[])
    }

    #[test]
    fn arrows_follow_application_cursor_mode() {
        assert_eq!(send(key("up")).unwrap(), b"\x1b[A");
        let app = KeyModes {
            app_cursor: true,
            ..KeyModes::default()
        };
        assert_eq!(encode(key("up"), app, &[]).unwrap(), b"\x1bOA");
        assert_eq!(encode(key("left"), app, &[]).unwrap(), b"\x1bOD");
    }

    #[test]
    fn modified_navigation_keys() {
        let shift_up = KeyPress {
            shift: true,
            ..key("up")
        };
        assert_eq!(send(shift_up).unwrap(), b"\x1b[1;2A");
        let ctrl_right = KeyPress {
            control: true,
            ..key("right")
        };
        assert_eq!(send(ctrl_right).unwrap(), b"\x1b[1;5C");
        let alt_delete = KeyPress {
            alt: true,
            ..key("delete")
        };
        assert_eq!(send(alt_delete).unwrap(), b"\x1b[3;3~");
        assert_eq!(send(key("pageup")).unwrap(), b"\x1b[5~");
        assert_eq!(send(key("home")).unwrap(), b"\x1b[H");
    }

    #[test]
    fn function_keys() {
        assert_eq!(send(key("f1")).unwrap(), b"\x1bOP");
        assert_eq!(send(key("f5")).unwrap(), b"\x1b[15~");
        assert_eq!(send(key("f12")).unwrap(), b"\x1b[24~");
        let shift_f3 = KeyPress {
            shift: true,
            ..key("f3")
        };
        assert_eq!(send(shift_f3).unwrap(), b"\x1b[1;2R");
    }

    #[test]
    fn editing_keys() {
        assert_eq!(send(key("enter")).unwrap(), b"\r");
        assert_eq!(send(key("tab")).unwrap(), b"\t");
        let shift_tab = KeyPress {
            shift: true,
            ..key("tab")
        };
        assert_eq!(send(shift_tab).unwrap(), b"\x1b[Z");
        assert_eq!(send(key("backspace")).unwrap(), b"\x7f");
        assert_eq!(send(key("escape")).unwrap(), b"\x1b");
    }

    #[test]
    fn control_combinations() {
        let ctrl = |name| KeyPress {
            control: true,
            ..key(name)
        };
        assert_eq!(send(ctrl("c")).unwrap(), [0x03]);
        assert_eq!(send(ctrl("a")).unwrap(), [0x01]);
        assert_eq!(send(ctrl("[")).unwrap(), [0x1b]);
        assert_eq!(send(ctrl("\\")).unwrap(), [0x1c]);
        assert_eq!(send(ctrl("space")).unwrap(), [0x00]);
        assert_eq!(send(ctrl("backspace")).unwrap(), [0x08]);
    }

    #[test]
    fn alt_is_meta_only_when_configured() {
        let alt_b = KeyPress {
            alt: true,
            ..key("b")
        };
        assert_eq!(send(alt_b), None, "Option composes characters by default");
        let meta = KeyModes {
            alt_is_meta: true,
            ..KeyModes::default()
        };
        assert_eq!(encode(alt_b, meta, &[]).unwrap(), b"\x1bb");
        let alt_shift_b = KeyPress {
            shift: true,
            ..alt_b
        };
        assert_eq!(encode(alt_shift_b, meta, &[]).unwrap(), b"\x1bB");
        let alt_enter = KeyPress {
            alt: true,
            ..key("enter")
        };
        assert_eq!(encode(alt_enter, meta, &[]).unwrap(), b"\x1b\r");
    }

    #[test]
    fn printable_text_and_app_shortcuts_are_not_encoded() {
        assert_eq!(send(key("a")), None);
        assert_eq!(send(key("space")), None);
        let cmd_c = KeyPress {
            platform: true,
            ..key("c")
        };
        assert_eq!(send(cmd_c), None);
    }

    #[test]
    fn bindings_win_including_shift_enter_newline() {
        let bindings = [Binding {
            key: "enter".into(),
            control: false,
            alt: false,
            shift: true,
            platform: false,
            chars: b"\x1b\r".to_vec(),
        }];
        let shift_enter = KeyPress {
            shift: true,
            ..key("enter")
        };
        assert_eq!(
            encode(shift_enter, KeyModes::default(), &bindings).unwrap(),
            b"\x1b\r"
        );
        assert_eq!(
            encode(key("enter"), KeyModes::default(), &bindings).unwrap(),
            b"\r"
        );
    }

    #[test]
    fn dropped_paths_are_escaped_and_spaced() {
        use std::path::PathBuf;
        let screenshot = PathBuf::from("/Users/me/Desktop/Screenshot 2026-10-23 at 14.29.43.png");
        assert_eq!(
            dropped_paths(&[screenshot]),
            "/Users/me/Desktop/Screenshot\\ 2026-10-23\\ at\\ 14.29.43.png "
        );
        let several = [
            PathBuf::from("/tmp/a.png"),
            PathBuf::from("/tmp/it's (1)&$x.txt"),
            PathBuf::from("/tmp/Résumé 日本.pdf"),
        ];
        assert_eq!(
            dropped_paths(&several),
            "/tmp/a.png /tmp/it\\'s\\ \\(1\\)\\&\\$x.txt /tmp/Résumé\\ 日本.pdf ",
            "shell characters escaped, other letters kept"
        );
        assert_eq!(dropped_paths(&[]), "");
    }

    #[test]
    fn paste_brackets_when_asked() {
        assert_eq!(paste("a\nb", false), b"a\rb");
        assert_eq!(paste("a\nb", true), b"\x1b[200~a\nb\x1b[201~");
        assert_eq!(
            paste("x\x1b[201~y", true),
            b"\x1b[200~xy\x1b[201~",
            "an end marker inside the paste is removed"
        );
    }
}
