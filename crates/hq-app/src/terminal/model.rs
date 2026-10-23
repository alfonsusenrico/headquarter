//! The terminal state of one attached session: `alacritty_terminal` fed by the host's
//! output frames. Replies the terminal owes the agent (device attributes, cursor
//! position, colour queries) are collected and returned for sending as input.

use std::sync::{Arc, Mutex};

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::{ClipboardType, Config, Term, TermMode};
use alacritty_terminal::vte::ansi::{CursorShape, CursorStyle, Processor, Rgb, StdSyncHandler};

use crate::theme::Palette;

/// Lines of output kept above the screen.
pub const SCROLLBACK_LINES: usize = 10_000;

/// Columns and rows of the visible screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridSize {
    pub columns: usize,
    pub rows: usize,
}

impl Dimensions for GridSize {
    fn total_lines(&self) -> usize {
        self.rows
    }

    fn screen_lines(&self) -> usize {
        self.rows
    }

    fn columns(&self) -> usize {
        self.columns
    }
}

/// Text the agent asked the terminal to copy (OSC 52), for the clipboard.
pub type AgentCopy = (ClipboardType, String);

/// Receives the terminal's events. Replies and the agent's copies matter here: the host
/// tracks status itself, so titles and bells are ignored.
#[derive(Clone)]
pub struct Listener {
    replies: Arc<Mutex<Vec<u8>>>,
    copies: Arc<Mutex<Vec<AgentCopy>>>,
    palette: Arc<Mutex<Palette>>,
}

impl EventListener for Listener {
    fn send_event(&self, event: Event) {
        let reply = match event {
            Event::PtyWrite(text) => text,
            // Only copying is accepted (alacritty_terminal's default `Osc52::OnlyCopy`):
            // an agent can set the clipboard but never read it (fix-agent-clipboard-copy).
            Event::ClipboardStore(target, text) => {
                self.copies
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push((target, text));
                return;
            }
            Event::ColorRequest(index, format) => {
                let palette = self
                    .palette
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                format(palette_color(&palette, index))
            }
            _ => return,
        };
        self.replies
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(reply.as_bytes());
    }
}

/// The palette colour for a colour index as alacritty numbers them: 0–255 indexed,
/// 256 foreground, 257 background, 258 cursor.
pub fn palette_color(palette: &Palette, index: usize) -> Rgb {
    match index {
        0..=15 => palette.ansi[index],
        16..=231 => {
            let cube = index - 16;
            let level = |value: usize| -> u8 {
                if value == 0 {
                    0
                } else {
                    u8::try_from(55 + value * 40).unwrap_or(u8::MAX)
                }
            };
            Rgb {
                r: level(cube / 36),
                g: level((cube / 6) % 6),
                b: level(cube % 6),
            }
        }
        232..=255 => {
            let gray = u8::try_from(8 + (index - 232) * 10).unwrap_or(u8::MAX);
            Rgb {
                r: gray,
                g: gray,
                b: gray,
            }
        }
        257 => palette.background,
        258 => palette.cursor,
        _ => palette.foreground,
    }
}

fn config(cursor: Option<CursorShape>) -> Config {
    Config {
        scrolling_history: SCROLLBACK_LINES,
        default_cursor_style: CursorStyle {
            shape: cursor.unwrap_or(CursorShape::Block),
            blinking: false,
        },
        ..Config::default()
    }
}

pub struct TerminalModel {
    term: Term<Listener>,
    processor: Processor<StdSyncHandler>,
    replies: Arc<Mutex<Vec<u8>>>,
    copies: Arc<Mutex<Vec<AgentCopy>>>,
    palette: Arc<Mutex<Palette>>,
    size: GridSize,
}

impl TerminalModel {
    pub fn new(size: GridSize, palette: Palette) -> TerminalModel {
        let replies = Arc::new(Mutex::new(Vec::new()));
        let copies = Arc::new(Mutex::new(Vec::new()));
        let palette = Arc::new(Mutex::new(palette));
        let listener = Listener {
            replies: Arc::clone(&replies),
            copies: Arc::clone(&copies),
            palette: Arc::clone(&palette),
        };
        TerminalModel {
            term: Term::new(config(None), &size, listener),
            processor: Processor::new(),
            replies,
            copies,
            palette,
            size,
        }
    }

    /// Feeds agent output and returns the bytes the terminal must send back.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.processor.advance(&mut self.term, bytes);
        std::mem::take(
            &mut *self
                .replies
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    /// What the agent asked to copy since the last call, oldest first.
    pub fn take_copies(&mut self) -> Vec<AgentCopy> {
        std::mem::take(
            &mut *self
                .copies
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    }

    pub fn resize(&mut self, size: GridSize) {
        if size != self.size && size.columns > 0 && size.rows > 0 {
            self.size = size;
            self.term.resize(size);
        }
    }

    pub fn size(&self) -> GridSize {
        self.size
    }

    /// The cursor shape used until the agent sets one; `None` is the block.
    pub fn set_default_cursor(&mut self, shape: Option<CursorShape>) {
        self.term.set_options(config(shape));
    }

    pub fn set_palette(&mut self, palette: Palette) {
        *self
            .palette
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = palette;
    }

    pub fn palette(&self) -> Palette {
        self.palette
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    pub fn term(&self) -> &Term<Listener> {
        &self.term
    }

    pub fn mode(&self) -> TermMode {
        *self.term.mode()
    }

    /// Scrolls the view by `lines` (positive is back into history).
    pub fn scroll(&mut self, lines: i32) {
        self.term.scroll_display(Scroll::Delta(lines));
    }

    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    /// Starts a selection at a visible cell (row 0 is the top of the view).
    /// `side` is the half of the cell the pointer is over, so a selection made leftwards
    /// or upwards still takes the first cell in, as in Alacritty.
    pub fn start_selection(&mut self, row: usize, column: usize, side: Side, kind: SelectionType) {
        let point = self.view_point(row, column);
        self.term.selection = Some(Selection::new(kind, point, side));
    }

    pub fn extend_selection(&mut self, row: usize, column: usize, side: Side) {
        let point = self.view_point(row, column);
        if let Some(selection) = self.term.selection.as_mut() {
            selection.update(point, side);
        }
    }

    pub fn clear_selection(&mut self) {
        self.term.selection = None;
    }

    /// Whether the selection starts after the first column of its row, so its first
    /// line lacks the row's margin.
    pub fn selection_starts_mid_row(&self) -> bool {
        self.term
            .selection
            .as_ref()
            .and_then(|selection| selection.to_range(&self.term))
            .is_some_and(|range| range.start.column.0 > 0)
    }

    pub fn selected_text(&self) -> Option<String> {
        self.term
            .selection_to_string()
            .filter(|text| !text.is_empty())
    }

    fn view_point(&self, row: usize, column: usize) -> Point {
        let offset = i32::try_from(self.term.grid().display_offset()).unwrap_or(0);
        let row = i32::try_from(row.min(self.size.rows.saturating_sub(1))).unwrap_or(0);
        Point::new(
            Line(row - offset),
            Column(column.min(self.size.columns.saturating_sub(1))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_selection_made_leftwards_takes_the_first_cell() {
        let mut model = model();
        model.feed(b"  two lines\r\n  of text");
        // From the end of the second line back to the very left of the first.
        model.start_selection(1, 8, Side::Right, SelectionType::Simple);
        model.extend_selection(0, 0, Side::Left);
        assert_eq!(
            model.selected_text().as_deref(),
            Some("  two lines\n  of text")
        );
    }
    use crate::theme::Theme;

    fn model() -> TerminalModel {
        TerminalModel::new(
            GridSize {
                columns: 20,
                rows: 5,
            },
            Theme::light().palette,
        )
    }

    fn text_of_row(model: &TerminalModel, row: usize) -> String {
        let grid = model.term().grid();
        let line = &grid[Line(i32::try_from(row).unwrap())];
        (0..model.size().columns)
            .map(|column| line[Column(column)].c)
            .collect::<String>()
            .trim_end()
            .to_owned()
    }

    #[test]
    fn output_lands_in_the_grid() {
        let mut model = model();
        assert!(model.feed(b"hello\r\nworld").is_empty());
        assert_eq!(text_of_row(&model, 0), "hello");
        assert_eq!(text_of_row(&model, 1), "world");
    }

    #[test]
    fn agent_copies_are_kept_and_reads_are_refused() {
        let mut model = model();
        // OSC 52 as Claude Code sends it: "hello" in base64, for the clipboard.
        assert!(model.feed(b"\x1b]52;c;aGVsbG8=\x07").is_empty());
        assert_eq!(
            model.take_copies(),
            [(ClipboardType::Clipboard, "hello".to_owned())]
        );
        assert!(model.take_copies().is_empty(), "each copy is taken once");
        assert!(
            model.feed(b"\x1b]52;c;?\x07").is_empty(),
            "a request to read the clipboard gets no reply"
        );
        assert!(model.take_copies().is_empty());
    }

    #[test]
    fn cursor_position_and_device_attribute_queries_are_answered() {
        let mut model = model();
        let reply = model.feed(b"abc\x1b[6n");
        assert_eq!(reply, b"\x1b[1;4R");
        let attributes = model.feed(b"\x1b[c");
        assert!(attributes.starts_with(b"\x1b[?"), "{attributes:?}");
    }

    #[test]
    fn background_colour_query_uses_the_theme() {
        let mut model = model();
        let reply = String::from_utf8(model.feed(b"\x1b]11;?\x07")).unwrap();
        assert!(reply.contains("rgb:ffff/ffff/ffff"), "{reply}");
        model.set_palette(Theme::dark().palette);
        let dark = String::from_utf8(model.feed(b"\x1b]11;?\x07")).unwrap();
        assert!(dark.contains("rgb:0f0f/1111/1515"), "{dark}");
    }

    #[test]
    fn alternate_screen_and_its_return() {
        let mut model = model();
        model.feed(b"shell prompt");
        model.feed(b"\x1b[?1049h\x1b[Hfull screen app");
        assert!(model.mode().contains(TermMode::ALT_SCREEN));
        assert_eq!(text_of_row(&model, 0), "full screen app");
        model.feed(b"\x1b[?1049l");
        assert!(!model.mode().contains(TermMode::ALT_SCREEN));
        assert_eq!(text_of_row(&model, 0), "shell prompt");
    }

    #[test]
    fn modes_agents_set_are_visible() {
        let mut model = model();
        model.feed(b"\x1b[?1h\x1b[?2004h\x1b[?1006h\x1b[?1000h");
        let mode = model.mode();
        assert!(mode.contains(TermMode::APP_CURSOR));
        assert!(mode.contains(TermMode::BRACKETED_PASTE));
        assert!(mode.contains(TermMode::SGR_MOUSE));
        assert!(mode.contains(TermMode::MOUSE_REPORT_CLICK));
    }

    #[test]
    fn resize_keeps_content_and_ignores_empty_sizes() {
        let mut model = model();
        model.feed(b"keep me");
        model.resize(GridSize {
            columns: 40,
            rows: 10,
        });
        assert_eq!(model.size().columns, 40);
        assert_eq!(text_of_row(&model, 0), "keep me");
        model.resize(GridSize {
            columns: 0,
            rows: 10,
        });
        assert_eq!(model.size().columns, 40);
    }

    #[test]
    fn selection_returns_text() {
        let mut model = model();
        model.feed(b"select this text");
        model.start_selection(0, 7, Side::Left, SelectionType::Simple);
        model.extend_selection(0, 10, Side::Right);
        assert_eq!(model.selected_text().as_deref(), Some("this"));
        model.clear_selection();
        assert_eq!(model.selected_text(), None);
    }

    #[test]
    fn scrollback_keeps_history() {
        let mut model = model();
        for line in 0..20 {
            model.feed(format!("line {line}\r\n").as_bytes());
        }
        model.scroll(5);
        assert_eq!(model.term().grid().display_offset(), 5);
        model.scroll_to_bottom();
        assert_eq!(model.term().grid().display_offset(), 0);
    }

    #[test]
    fn indexed_colours_follow_xterm() {
        let palette = Theme::light().palette;
        assert_eq!(palette_color(&palette, 1), palette.ansi[1]);
        assert_eq!(palette_color(&palette, 196), Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(palette_color(&palette, 16), Rgb { r: 0, g: 0, b: 0 });
        assert_eq!(palette_color(&palette, 232), Rgb { r: 8, g: 8, b: 8 });
        assert_eq!(palette_color(&palette, 257), palette.background);
    }

    /// 10 MB of coloured agent-like output through the model, timed. Run in release
    /// with `cargo test --release -p hq-app -- --ignored --nocapture throughput`.
    #[test]
    #[ignore = "timing evidence, not a check"]
    fn throughput_of_ten_megabytes() {
        let mut model = TerminalModel::new(
            GridSize {
                columns: 120,
                rows: 40,
            },
            Theme::dark().palette,
        );
        let line = "\x1b[32m+ added\x1b[0m \x1b[1mbold\x1b[0m plain text for a long agent line, \u{4e2d}\u{6587} \x1b[38;2;200;120;50mtruecolor\x1b[0m\r\n";
        let chunk = line.repeat(64 * 1024 / line.len() + 1);
        let total = 10 * 1024 * 1024;
        let started = std::time::Instant::now();
        let mut fed = 0;
        while fed < total {
            model.feed(chunk.as_bytes());
            fed += chunk.len();
        }
        let elapsed = started.elapsed();
        let mb = fed as f64 / (1024.0 * 1024.0);
        println!(
            "throughput: {mb:.1} MB in {:.3} s = {:.1} MB/s",
            elapsed.as_secs_f64(),
            mb / elapsed.as_secs_f64()
        );
        let started = std::time::Instant::now();
        for _ in 0..1000 {
            std::hint::black_box(crate::terminal::frame::build(&model));
        }
        println!(
            "frame build: 120x40 screen in {:.1} us",
            started.elapsed().as_secs_f64() * 1000.0
        );
    }
}
