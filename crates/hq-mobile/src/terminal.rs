//! One open session or shell on the phone (add-android-app D8): `alacritty_terminal` fed
//! by the host's output frames, with scrollback, and snapshots for Kotlin to draw.
//! Colours stay symbolic for the 16 theme colours and the defaults, so the app's light
//! and dark tokens decide them.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::Column;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor, StdSyncHandler};
use hq_links::LinkFinder;

/// Lines of output kept above the screen.
const SCROLLBACK_LINES: usize = 2000;
/// The most wheel reports or arrows one scroll sends, as the desktop's wheel.
const SCROLL_REPORTS: u32 = 10;
/// How long a scroll batch counts as on its way without an answer, in case the answer never
/// comes (fix-phone-scroll-link D5).
const BATCH_STALE: Duration = Duration::from_secs(2);
/// The most drag lines kept waiting for the next batch: two batches, so a drag that outruns the
/// link scrolls in bigger steps and stops soon after the finger.
const SCROLL_KEPT: i32 = 2 * SCROLL_REPORTS as i32;

/// What a drag on the terminal did (fix-phone-scroll-link D5).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Dragged {
    /// The phone's own history scrolled.
    Local,
    /// The lines wait for the batch on its way to be answered.
    Waiting,
    /// A batch for the agent to send now.
    Send(Vec<u8>),
}

/// Drag lines for a full-screen agent, sent one batch at a time: the next batch waits until
/// the computer has answered after the last one, so a slow link never queues repaints
/// (fix-phone-scroll-link D5).
#[derive(Debug, Default)]
struct ScrollBatch {
    pending: i32,
    row: u16,
    col: u16,
    /// When the batch on its way was sent.
    out_since: Option<Instant>,
}

impl ScrollBatch {
    fn add(&mut self, lines: i32, row: u16, col: u16) {
        self.pending = (self.pending + lines).clamp(-SCROLL_KEPT, SCROLL_KEPT);
        self.row = row;
        self.col = col;
    }

    /// The lines to send now, at most one batch: none while a batch is on its way.
    fn take(&mut self, now: Instant) -> Option<(i32, u16, u16)> {
        let waiting = self
            .out_since
            .is_some_and(|since| now.duration_since(since) < BATCH_STALE);
        if waiting || self.pending == 0 {
            return None;
        }
        let most = SCROLL_REPORTS as i32;
        let lines = self.pending.clamp(-most, most);
        self.pending -= lines;
        self.out_since = Some(now);
        Some((lines, self.row, self.col))
    }

    fn answered(&mut self) {
        self.out_since = None;
    }
}

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
    Left,
    Right,
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
    links: LinkFinder,
    batch: ScrollBatch,
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
            links: LinkFinder::new(),
            batch: ScrollBatch::default(),
        }
    }

    /// A drag of `lines` at a cell. The phone's own history scrolls at once. For a full-screen
    /// agent the lines join the next batch, which goes when the batch before it was answered.
    pub fn drag(&mut self, lines: i32, row: u16, col: u16, now: Instant) -> Dragged {
        if lines == 0 {
            return Dragged::Waiting;
        }
        if !self.scrolls_agent() {
            self.scroll(lines, row, col);
            return Dragged::Local;
        }
        self.batch.add(lines, row, col);
        self.next_batch(now).map_or(Dragged::Waiting, Dragged::Send)
    }

    /// The computer answered after the last batch: the lines waiting since may go.
    pub fn batch_answered(&mut self, now: Instant) -> Option<Vec<u8>> {
        self.batch.answered();
        self.next_batch(now)
    }

    fn next_batch(&mut self, now: Instant) -> Option<Vec<u8>> {
        let (lines, row, col) = self.batch.take(now)?;
        let bytes = self.scroll(lines, row, col);
        (!bytes.is_empty()).then_some(bytes)
    }

    /// Whether a drag goes to the agent (wheel reports or arrows) rather than the history.
    fn scrolls_agent(&self) -> bool {
        let mode = *self.term.mode();
        (mode.intersects(TermMode::MOUSE_MODE) && mode.contains(TermMode::SGR_MOUSE))
            || mode.contains(TermMode::ALT_SCREEN)
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

    /// The web address at a cell of the screen as drawn, scrolled back or not
    /// (refine-phone-typing-and-links D4).
    pub fn link_at(&mut self, row: u16, col: u16) -> Option<String> {
        let point = hq_links::view_point(&self.term, usize::from(row), usize::from(col));
        self.links
            .at(&self.term, point)
            .map(|link| link.address().to_owned())
    }

    /// The arrow presses that bring the agent's cursor to a tapped cell on the cursor's own row:
    /// negative for ←, positive for →, counted in characters (a wide character is one press).
    /// A tap past the row's text stops at its end. None when the tap is on another row, the
    /// cursor is hidden, or the view is scrolled back (add-phone-cursor-movement D2).
    pub fn cursor_steps(&self, row: u16, col: u16) -> Option<i32> {
        if self.term.grid().display_offset() != 0
            || !self.term.mode().contains(TermMode::SHOW_CURSOR)
        {
            return None;
        }
        let cursor = self.term.grid().cursor.point;
        if i32::from(row) != cursor.line.0 {
            return None;
        }
        let line = &self.term.grid()[cursor.line];
        let columns = self.term.columns();
        let text_end = (0..columns)
            .rev()
            .find(|&column| !matches!(line[Column(column)].c, ' ' | '\0'))
            .map_or(0, |last| last + 1);
        let from = cursor.column.0;
        let mut to = usize::from(col).min(columns);
        if to > from {
            to = to.min(text_end.max(from));
        }
        let (low, high) = if to < from { (to, from) } else { (from, to) };
        let presses = (low..high)
            .filter(|&column| {
                !line[Column(column)]
                    .flags
                    .intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER)
            })
            .count() as i32;
        Some(if to < from { -presses } else { presses })
    }

    /// Whether the program asked for pastes to be bracketed.
    pub fn bracketed_paste(&self) -> bool {
        self.term.mode().contains(TermMode::BRACKETED_PASTE)
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
            Key::Left if app_cursor => b"\x1bOD",
            Key::Left => b"\x1b[D",
            Key::Right if app_cursor => b"\x1bOC",
            Key::Right => b"\x1b[C",
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

    /// A terminal whose agent asked for SGR wheel reports, as Claude Code's fullscreen mode does.
    fn full_screen() -> Terminal {
        let mut terminal = Terminal::new(40, 10);
        terminal.feed(b"\x1b[?1049h\x1b[?1000h\x1b[?1006h");
        terminal
    }

    fn reports(bytes: &[u8]) -> usize {
        String::from_utf8_lossy(bytes).matches("\x1b[<").count()
    }

    #[test]
    fn one_scroll_batch_is_on_its_way_at_a_time() {
        let mut terminal = full_screen();
        let start = Instant::now();
        let Dragged::Send(first) = terminal.drag(2, 5, 5, start) else {
            panic!("the first drag goes at once");
        };
        assert_eq!(reports(&first), 2);
        assert_eq!(terminal.drag(3, 5, 5, start), Dragged::Waiting);
        assert_eq!(terminal.drag(4, 5, 5, start), Dragged::Waiting);
        let next = terminal
            .batch_answered(start)
            .expect("the waiting lines go");
        assert_eq!(reports(&next), 7, "both drags in one batch");
        assert_eq!(terminal.batch_answered(start), None, "nothing waits");
    }

    #[test]
    fn a_batch_is_at_most_ten_reports_and_two_are_kept() {
        let mut terminal = full_screen();
        let start = Instant::now();
        let Dragged::Send(first) = terminal.drag(1, 5, 5, start) else {
            panic!("sent");
        };
        assert_eq!(reports(&first), 1);
        for _ in 0..50 {
            terminal.drag(1, 5, 5, start);
        }
        let second = terminal.batch_answered(start).unwrap();
        assert_eq!(reports(&second), 10, "one batch is ten reports");
        let third = terminal.batch_answered(start).unwrap();
        assert_eq!(reports(&third), 10, "a second batch was kept");
        assert_eq!(
            terminal.batch_answered(start),
            None,
            "the rest of a fast drag is dropped"
        );
    }

    #[test]
    fn an_unanswered_batch_stops_holding_after_two_seconds() {
        let mut terminal = full_screen();
        let start = Instant::now();
        assert!(matches!(terminal.drag(1, 5, 5, start), Dragged::Send(_)));
        assert_eq!(
            terminal.drag(1, 5, 5, start + Duration::from_secs(1)),
            Dragged::Waiting
        );
        assert!(matches!(
            terminal.drag(1, 5, 5, start + Duration::from_secs(3)),
            Dragged::Send(_)
        ));
    }

    #[test]
    fn opposite_drags_cancel_out_and_history_scrolls_at_once() {
        let mut terminal = full_screen();
        let start = Instant::now();
        assert!(matches!(terminal.drag(3, 5, 5, start), Dragged::Send(_)));
        terminal.drag(2, 5, 5, start);
        terminal.drag(-2, 5, 5, start);
        assert_eq!(terminal.batch_answered(start), None, "they cancelled");
        let mut inline = Terminal::new(40, 10);
        assert_eq!(inline.drag(3, 5, 5, start), Dragged::Local);
    }

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
    fn a_tap_on_the_cursors_row_counts_the_arrows_to_it() {
        let mut terminal = Terminal::new(30, 4);
        // A prompt, then "abcdef" with the cursor after it, as Claude Code draws it.
        terminal.feed(b"> abcdef");
        assert_eq!(terminal.cursor_steps(0, 4), Some(-4), "to the c");
        assert_eq!(terminal.cursor_steps(0, 8), Some(0), "already there");
        assert_eq!(
            terminal.cursor_steps(0, 25),
            Some(0),
            "past the text stops at its end"
        );
        assert_eq!(terminal.cursor_steps(1, 4), None, "another row");
        terminal.feed(b"\x1b[4D");
        assert_eq!(
            terminal.cursor_steps(0, 7),
            Some(3),
            "right, to before the f"
        );
        assert_eq!(terminal.cursor_steps(0, 25), Some(4), "to the end");
        terminal.feed(b"\x1b[?25l");
        assert_eq!(terminal.cursor_steps(0, 2), None, "the cursor is hidden");
    }

    #[test]
    fn a_wide_character_is_one_press() {
        let mut terminal = Terminal::new(30, 4);
        terminal.feed("> 日本x".as_bytes());
        // Cells: "> " 0-1, 日 2-3, 本 4-5, x 6, the cursor at 7.
        assert_eq!(terminal.cursor_steps(0, 2), Some(-3));
        assert_eq!(
            terminal.cursor_steps(0, 3),
            Some(-2),
            "the second half of 日 rounds right"
        );
    }

    #[test]
    fn left_and_right_follow_the_cursor_mode() {
        let mut terminal = Terminal::new(10, 3);
        assert_eq!(terminal.key_bytes(Key::Left), b"\x1b[D");
        assert_eq!(terminal.key_bytes(Key::Right), b"\x1b[C");
        terminal.feed(b"\x1b[?1h");
        assert_eq!(terminal.key_bytes(Key::Left), b"\x1bOD");
        assert_eq!(terminal.key_bytes(Key::Right), b"\x1bOC");
    }

    #[test]
    fn a_tapped_cell_gives_its_web_link() {
        let mut terminal = Terminal::new(20, 4);
        terminal.feed(b"go https://example.com/long/path ok\r\n");
        terminal.feed(b"\x1b]8;;file:///etc/hosts\x07hosts\x1b]8;;\x07");
        let whole = Some("https://example.com/long/path".to_owned());
        assert_eq!(terminal.link_at(0, 10), whole);
        assert_eq!(
            terminal.link_at(1, 3),
            whole,
            "the row the terminal wrapped"
        );
        assert_eq!(terminal.link_at(0, 1), None, "beside the link");
        assert_eq!(terminal.link_at(2, 1), None, "not a web link");
    }

    #[test]
    fn a_link_is_found_where_it_is_drawn_while_scrolled_back() {
        let mut terminal = Terminal::new(30, 3);
        terminal.feed(b"\x1b]8;;https://example.com/docs\x07docs\x1b]8;;\x07\r\n");
        for line in 0..4 {
            terminal.feed(format!("line {line}\r\n").as_bytes());
        }
        assert_eq!(terminal.link_at(0, 1), None);
        terminal.scroll(3, 0, 0);
        assert_eq!(terminal.screen().scrolled_back, 3);
        assert_eq!(
            terminal.link_at(0, 1).as_deref(),
            Some("https://example.com/docs")
        );
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
