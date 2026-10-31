//! One open session or shell on the phone (add-android-app D8): `alacritty_terminal` fed
//! by the host's output frames, with scrollback, and snapshots for Kotlin to draw.
//! Colours stay symbolic for the 16 theme colours and the defaults, so the app's light
//! and dark tokens decide them.

use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor, StdSyncHandler};

/// Lines of output kept above the screen.
const SCROLLBACK_LINES: usize = 2000;
/// The most wheel reports or arrows one scroll sends, as the desktop's wheel.
const SCROLL_REPORTS: u32 = 10;

/// A colour as the app resolves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Paint {
    Foreground,
    Background,
    /// One of the theme's 16 colours (0-7 normal, 8-15 bright).
    Ansi {
        index: u8,
    },
    /// A colour the agent gave exactly, or one of the 240 fixed colours: `0xRRGGBB`.
    Rgb {
        rgb: u32,
    },
}

/// A run of cells with one style, starting at `col`, `cells` wide.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Span {
    pub col: u16,
    pub cells: u16,
    pub text: String,
    pub fg: Paint,
    pub bg: Paint,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct Screen {
    pub cols: u16,
    pub rows: u16,
    /// The visible lines, top first.
    pub lines: Vec<Vec<Span>>,
    pub cursor_row: u16,
    pub cursor_col: u16,
    /// False while the agent hides the cursor or the view is scrolled back.
    pub cursor_visible: bool,
    /// How many lines the view is scrolled back. 0 follows the output.
    pub scrolled_back: u32,
    /// Lines of history above the screen.
    pub history: u32,
}

/// Keys of the key row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum Key {
    One,
    Two,
    Three,
    Escape,
    ShiftTab,
    Up,
    Down,
    Tab,
    CtrlC,
    CtrlD,
    Enter,
}

#[derive(Clone, Copy)]
struct Size {
    cols: usize,
    rows: usize,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.cols
    }
}

/// Collects what the terminal owes the agent (device attributes, cursor position), sent
/// back as input like any terminal's replies.
#[derive(Clone)]
struct Replies(Arc<Mutex<Vec<u8>>>);

impl EventListener for Replies {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(text) = event {
            self.0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .extend_from_slice(text.as_bytes());
        }
    }
}

pub(crate) struct Terminal {
    term: Term<Replies>,
    processor: Processor<StdSyncHandler>,
    replies: Arc<Mutex<Vec<u8>>>,
    size: Size,
}

impl Terminal {
    pub fn new(cols: u16, rows: u16) -> Terminal {
        let size = Size {
            cols: usize::from(cols.max(1)),
            rows: usize::from(rows.max(1)),
        };
        let replies = Arc::new(Mutex::new(Vec::new()));
        let config = Config {
            scrolling_history: SCROLLBACK_LINES,
            ..Config::default()
        };
        Terminal {
            term: Term::new(config, &size, Replies(Arc::clone(&replies))),
            processor: Processor::new(),
            replies,
            size,
        }
    }

    /// Feeds agent output. Returns the replies the terminal owes the agent.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.processor.advance(&mut self.term, bytes);
        std::mem::take(
            &mut *self
                .replies
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let size = Size {
            cols: usize::from(cols.max(1)),
            rows: usize::from(rows.max(1)),
        };
        if (size.cols, size.rows) != (self.size.cols, self.size.rows) {
            self.size = size;
            self.term.resize(size);
        }
    }

    pub fn size(&self) -> (u16, u16) {
        (self.size.cols as u16, self.size.rows as u16)
    }

    /// Scrolls by `lines` at the cell `row`, `col`, as the desktop's wheel does
    /// (fix-phone-scroll D1): wheel reports when the agent asked for SGR mouse reports,
    /// arrow keys on the alternate screen, otherwise this terminal's own history. Positive
    /// goes back. Returns the bytes to send to the agent.
    pub fn scroll(&mut self, lines: i32, row: u16, col: u16) -> Vec<u8> {
        if lines == 0 {
            return Vec::new();
        }
        let mode = *self.term.mode();
        let count = lines.unsigned_abs().min(SCROLL_REPORTS) as usize;
        let back = lines > 0;
        if mode.intersects(TermMode::MOUSE_MODE) && mode.contains(TermMode::SGR_MOUSE) {
            let row = usize::from(row).min(self.size.rows - 1) + 1;
            let col = usize::from(col).min(self.size.cols - 1) + 1;
            let button = if back { 64 } else { 65 };
            return format!("\x1b[<{button};{col};{row}M")
                .repeat(count)
                .into_bytes();
        }
        if mode.contains(TermMode::ALT_SCREEN) {
            let key = if back { Key::Up } else { Key::Down };
            return self.key_bytes(key).repeat(count);
        }
        self.term.scroll_display(Scroll::Delta(lines));
        Vec::new()
    }

    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    /// The bytes a key sends, as the agent's current modes expect them.
    pub fn key_bytes(&self, key: Key) -> &'static [u8] {
        let app_cursor = self.term.mode().contains(TermMode::APP_CURSOR);
        match key {
            Key::One => b"1",
            Key::Two => b"2",
            Key::Three => b"3",
            Key::Escape => b"\x1b",
            Key::ShiftTab => b"\x1b[Z",
            Key::Up if app_cursor => b"\x1bOA",
            Key::Up => b"\x1b[A",
            Key::Down if app_cursor => b"\x1bOB",
            Key::Down => b"\x1b[B",
            Key::Tab => b"\t",
            Key::CtrlC => b"\x03",
            Key::CtrlD => b"\x04",
            Key::Enter => b"\r",
        }
    }

    pub fn screen(&self) -> Screen {
        let cols = self.term.columns();
        let rows = self.term.screen_lines();
        let mut lines: Vec<Vec<Span>> = (0..rows).map(|_| Vec::new()).collect();
        let content = self.term.renderable_content();
        let offset = content.display_offset as i32;
        for indexed in content.display_iter {
            let Ok(row) = usize::try_from(indexed.point.line.0 + offset) else {
                continue;
            };
            if row >= rows {
                continue;
            }
            let cell = indexed.cell;
            if cell
                .flags
                .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            {
                continue;
            }
            let (mut fg, mut bg) = (paint(cell.fg), paint(cell.bg));
            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg, &mut bg);
            }
            let style = (
                cell.flags.contains(Flags::BOLD),
                cell.flags.contains(Flags::ITALIC),
                cell.flags.intersects(Flags::ALL_UNDERLINES),
                cell.flags.contains(Flags::DIM),
            );
            let wide = cell.flags.contains(Flags::WIDE_CHAR);
            let col = indexed.point.column.0 as u16;
            let line = &mut lines[row];
            match line.last_mut() {
                Some(last)
                    if !wide
                        && last.fg == fg
                        && last.bg == bg
                        && (last.bold, last.italic, last.underline, last.dim) == style
                        && last.col + last.cells == col =>
                {
                    last.text.push(cell.c);
                    last.cells += 1;
                }
                _ => line.push(Span {
                    col,
                    cells: if wide { 2 } else { 1 },
                    text: cell.c.to_string(),
                    fg,
                    bg,
                    bold: style.0,
                    italic: style.1,
                    underline: style.2,
                    dim: style.3,
                }),
            }
        }
        let cursor = content.cursor.point;
        Screen {
            cols: cols as u16,
            rows: rows as u16,
            lines,
            cursor_row: u16::try_from(cursor.line.0 + offset).unwrap_or(0),
            cursor_col: cursor.column.0 as u16,
            cursor_visible: offset == 0 && self.term.mode().contains(TermMode::SHOW_CURSOR),
            scrolled_back: content.display_offset as u32,
            history: self.term.history_size() as u32,
        }
    }

    #[cfg(test)]
    pub fn text(&self) -> String {
        self.screen()
            .lines
            .iter()
            .map(|line| {
                let mut text = String::new();
                for span in line {
                    while text.chars().count() < usize::from(span.col) {
                        text.push(' ');
                    }
                    text.push_str(&span.text);
                }
                text.trim_end().to_owned()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn paint(color: Color) -> Paint {
    match color {
        Color::Spec(rgb) => Paint::Rgb {
            rgb: (u32::from(rgb.r) << 16) | (u32::from(rgb.g) << 8) | u32::from(rgb.b),
        },
        Color::Indexed(index) => indexed(index),
        Color::Named(NamedColor::Background) => Paint::Background,
        Color::Named(named) => match named as usize {
            index @ 0..16 => Paint::Ansi { index: index as u8 },
            _ => Paint::Foreground,
        },
    }
}

/// The 256-colour table above the theme's 16: a 6×6×6 cube, then 24 greys.
fn indexed(index: u8) -> Paint {
    match index {
        0..16 => Paint::Ansi { index },
        16..232 => {
            let i = u32::from(index - 16);
            let level = |v: u32| if v == 0 { 0 } else { 55 + v * 40 };
            Paint::Rgb {
                rgb: (level(i / 36) << 16) | (level((i / 6) % 6) << 8) | level(i % 6),
            }
        }
        _ => {
            let grey = 8 + u32::from(index - 232) * 10;
            Paint::Rgb {
                rgb: (grey << 16) | (grey << 8) | grey,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_lands_on_the_screen_with_its_colours() {
        let mut terminal = Terminal::new(20, 4);
        terminal
            .feed(b"plain \x1b[31mred\x1b[0m \x1b[38;2;1;2;3mexact\x1b[0m\r\n\x1b[1;7mbold\x1b[0m");
        assert_eq!(terminal.text(), "plain red exact\nbold\n\n");
        let screen = terminal.screen();
        let red = screen.lines[0]
            .iter()
            .find(|span| span.text == "red")
            .unwrap();
        assert_eq!(red.fg, Paint::Ansi { index: 1 });
        let exact = screen.lines[0]
            .iter()
            .find(|span| span.text == "exact")
            .unwrap();
        assert_eq!(exact.fg, Paint::Rgb { rgb: 0x010203 });
        let bold = &screen.lines[1][0];
        assert!(bold.bold);
        // Inverse swaps the defaults, so the app paints them from its theme.
        assert_eq!((bold.fg, bold.bg), (Paint::Background, Paint::Foreground));
        assert_eq!((screen.cursor_row, screen.cursor_col), (1, 4));
    }

    #[test]
    fn history_scrolls_back_and_hides_the_cursor() {
        let mut terminal = Terminal::new(10, 3);
        for line in 0..10 {
            terminal.feed(format!("line {line}\r\n").as_bytes());
        }
        assert_eq!(terminal.screen().history, 8);
        assert!(
            terminal.scroll(2, 0, 0).is_empty(),
            "nothing goes to the agent"
        );
        let screen = terminal.screen();
        assert_eq!(screen.scrolled_back, 2);
        assert!(!screen.cursor_visible);
        assert!(terminal.text().starts_with("line 6"), "{}", terminal.text());
        terminal.scroll_to_bottom();
        assert_eq!(terminal.screen().scrolled_back, 0);
    }

    /// Claude Code 2.1.312 draws on the alternate screen with SGR mouse reports.
    #[test]
    fn a_full_screen_agent_with_the_mouse_gets_wheel_reports_at_the_cell() {
        let mut terminal = Terminal::new(40, 10);
        terminal.feed(b"\x1b[?1049h\x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1006h");
        assert_eq!(terminal.scroll(2, 4, 7), b"\x1b[<64;8;5M\x1b[<64;8;5M");
        assert_eq!(terminal.scroll(-1, 4, 7), b"\x1b[<65;8;5M");
        assert_eq!(
            terminal.scroll(30, 99, 99),
            b"\x1b[<64;40;10M".repeat(10),
            "clamped to the grid, ten at most"
        );
        assert_eq!(terminal.screen().scrolled_back, 0);
    }

    #[test]
    fn a_full_screen_agent_without_the_mouse_gets_arrows() {
        let mut terminal = Terminal::new(40, 10);
        terminal.feed(b"\x1b[?1049h");
        assert_eq!(terminal.scroll(2, 0, 0), b"\x1b[A\x1b[A");
        terminal.feed(b"\x1b[?1h");
        assert_eq!(terminal.scroll(-1, 0, 0), b"\x1bOB");
        // Mouse reports in the old encoding are not used, as on the desktop.
        terminal.feed(b"\x1b[?1000h");
        assert_eq!(terminal.scroll(1, 0, 0), b"\x1bOA");
    }

    #[test]
    fn arrows_follow_the_cursor_mode_and_replies_are_returned() {
        let mut terminal = Terminal::new(10, 3);
        assert_eq!(terminal.key_bytes(Key::Up), b"\x1b[A");
        terminal.feed(b"\x1b[?1h");
        assert_eq!(terminal.key_bytes(Key::Up), b"\x1bOA");
        assert_eq!(terminal.key_bytes(Key::ShiftTab), b"\x1b[Z");
        // A cursor position query is answered.
        assert_eq!(terminal.feed(b"\x1b[6n"), b"\x1b[1;1R");
    }

    #[test]
    fn resizing_changes_the_grid() {
        let mut terminal = Terminal::new(10, 3);
        terminal.resize(54, 40);
        assert_eq!(terminal.size(), (54, 40));
        assert_eq!(terminal.screen().lines.len(), 40);
    }
}
