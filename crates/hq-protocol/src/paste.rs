//! What clients paste into a terminal, shared by the desktop app and the phone: text wrapped
//! for bracketed paste, and paths typed as a drop of files types them (add-terminal-file-drop,
//! add-image-upload D5).

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
/// word does not join it. Agents read an image path typed this way as an attachment. The
/// paths are text, as the computer that holds the files names them.
pub fn typed_paths<S: AsRef<str>>(paths: &[S]) -> String {
    let mut text = String::new();
    for path in paths {
        for c in path.as_ref().chars() {
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

    #[test]
    fn paths_are_escaped_and_spaced() {
        assert_eq!(
            typed_paths(&["/Users/me/Desktop/Screenshot 2026-10-23 at 14.29.43.png"]),
            "/Users/me/Desktop/Screenshot\\ 2026-10-23\\ at\\ 14.29.43.png "
        );
        assert_eq!(
            typed_paths(&["/tmp/a.png", "/tmp/it's (1)&$x.txt", "/tmp/Résumé 日本.pdf"]),
            "/tmp/a.png /tmp/it\\'s\\ \\(1\\)\\&\\$x.txt /tmp/Résumé\\ 日本.pdf ",
            "shell characters escaped, other letters kept"
        );
        assert_eq!(typed_paths::<&str>(&[]), "");
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
