//! The terminal modes in effect where a session's catch-up buffer starts. An agent sets
//! its modes once, at startup; when that output has left the buffer, a client attaching
//! later would never see them, so it is sent these first (fix-catch-up-terminal-modes).

use alacritty_terminal::vte::{Params, Parser, Perform};

/// Private modes (DECSET) a viewer needs to show and drive an agent, with their reset
/// state. Synchronized output (2026) is left out on purpose: replaying an update the
/// agent had not finished yet would hold the viewer's screen.
const TRACKED: [(u16, bool); 15] = [
    (1, false),    // application cursor keys
    (7, true),     // autowrap
    (12, false),   // cursor blinking
    (25, true),    // cursor visible
    (47, false),   // alternate screen
    (1047, false), // alternate screen
    (1049, false), // alternate screen, saving the cursor
    (1000, false), // mouse clicks
    (1002, false), // mouse drags
    (1003, false), // all mouse motion
    (1004, false), // focus reports
    (1005, false), // UTF-8 mouse encoding
    (1006, false), // SGR mouse encoding
    (1015, false), // urxvt mouse encoding
    (2004, false), // bracketed paste
];

pub struct Modes {
    parser: Parser,
    state: ModeState,
}

#[derive(Default)]
struct ModeState {
    /// Index into `TRACKED`: whether the mode is set.
    set: [Option<bool>; TRACKED.len()],
}

impl Modes {
    pub fn new() -> Modes {
        Modes {
            parser: Parser::new(),
            state: ModeState::default(),
        }
    }

    /// Follows output leaving the catch-up buffer, in order.
    pub fn observe(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.state, bytes);
    }

    /// The sequences that put a fresh terminal in the observed modes: only modes away
    /// from their reset state.
    pub fn preamble(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (index, &(mode, reset)) in TRACKED.iter().enumerate() {
            match self.state.set[index] {
                Some(set) if set != reset => {
                    let action = if set { 'h' } else { 'l' };
                    bytes.extend(format!("\x1b[?{mode}{action}").into_bytes());
                }
                _ => {}
            }
        }
        bytes
    }
}

impl Perform for ModeState {
    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, action: char) {
        if ignore || intermediates != b"?" || !matches!(action, 'h' | 'l') {
            return;
        }
        for param in params.iter() {
            let Some(&mode) = param.first() else {
                continue;
            };
            if let Some(index) = TRACKED.iter().position(|&(tracked, _)| tracked == mode) {
                self.set[index] = Some(action == 'h');
            }
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        // RIS (ESC c): a full reset puts every mode back.
        if intermediates.is_empty() && byte == b'c' {
            self.set = Default::default();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preamble_after(chunks: &[&[u8]]) -> String {
        let mut modes = Modes::new();
        for chunk in chunks {
            modes.observe(chunk);
        }
        String::from_utf8(modes.preamble()).unwrap()
    }

    #[test]
    fn modes_away_from_reset_are_replayed_in_a_fixed_order() {
        assert_eq!(
            preamble_after(&[b"\x1b[?2004h\x1b[?1049h\x1b[?1002;1006htext\x1b[?25l"]),
            "\x1b[?25l\x1b[?1049h\x1b[?1002h\x1b[?1006h\x1b[?2004h"
        );
    }

    #[test]
    fn the_last_word_counts_and_resets_are_not_replayed() {
        assert_eq!(
            preamble_after(&[b"\x1b[?1004h\x1b[?1000h", b"\x1b[?1004l\x1b[?7h"]),
            "\x1b[?1000h"
        );
    }

    #[test]
    fn a_sequence_split_across_chunks_still_counts() {
        assert_eq!(preamble_after(&[b"\x1b[?10", b"49h"]), "\x1b[?1049h");
    }

    #[test]
    fn synchronized_updates_and_unknown_modes_are_left_out() {
        assert_eq!(preamble_after(&[b"\x1b[?2026h\x1b[?9999h\x1b[4h"]), "");
    }

    #[test]
    fn a_full_reset_forgets_earlier_modes() {
        assert_eq!(
            preamble_after(&[b"\x1b[?1049h\x1b[?2004h\x1bc\x1b[?1h"]),
            "\x1b[?1h"
        );
    }
}
