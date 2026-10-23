//! A render-ready snapshot of the visible screen: every cell's character and resolved
//! colours, the cursor, and the runs the painter draws (merged backgrounds, styled text).

use alacritty_terminal::term::TermMode;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Rgb};

use super::model::{TerminalModel, palette_color};
use crate::theme::{Paint, Palette};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Underline {
    None,
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextStyle {
    pub color: Paint,
    pub bold: bool,
    pub italic: bool,
    pub underline: Underline,
    pub strike: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledCell {
    pub ch: char,
    /// Combining characters drawn on top of `ch`.
    pub zerowidth: Vec<char>,
    pub background: Paint,
    pub style: TextStyle,
    /// The first cell of a two-column character.
    pub wide: bool,
    /// The second cell of a two-column character: nothing is drawn here.
    pub spacer: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cursor {
    pub row: usize,
    pub column: usize,
    pub shape: CursorShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub columns: usize,
    pub rows: Vec<Vec<StyledCell>>,
    pub cursor: Option<Cursor>,
    pub background: Paint,
    pub cursor_color: Paint,
}

/// A rectangle of one background colour, in cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackgroundRun {
    pub row: usize,
    pub column: usize,
    pub cells: usize,
    pub color: Paint,
}

/// Text drawn from one column with one style. A wide character always has a run of
/// its own, so the painter can give it two cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextRun {
    pub row: usize,
    pub column: usize,
    pub text: String,
    pub cells: usize,
    pub style: TextStyle,
    pub wide: bool,
}

pub fn build(model: &TerminalModel) -> Frame {
    let term = model.term();
    let palette = model.palette();
    let size = model.size();
    let content = term.renderable_content();
    let colors = content.colors;
    // Colours an agent set at run time are opaque; palette colours carry the theme's
    // opacity.
    let resolve = |color: Color| -> Paint {
        let index = match color {
            Color::Spec(rgb) => return Paint::solid(rgb),
            Color::Indexed(index) => usize::from(index),
            Color::Named(named) => named as usize,
        };
        colors[index].map_or_else(
            || Paint::solid(palette_color(&palette, index)).with_opacity(palette.opacity_of(index)),
            Paint::solid,
        )
    };
    let background = resolve(Color::Named(NamedColor::Background));
    let blank = StyledCell {
        ch: ' ',
        zerowidth: Vec::new(),
        background,
        style: TextStyle {
            color: resolve(Color::Named(NamedColor::Foreground)),
            bold: false,
            italic: false,
            underline: Underline::None,
            strike: false,
        },
        wide: false,
        spacer: false,
    };
    let mut rows = vec![vec![blank; size.columns]; size.rows];
    let offset = content.display_offset;
    let selection = content.selection;

    for indexed in content.display_iter {
        let Ok(row) = usize::try_from(indexed.point.line.0 + i32::try_from(offset).unwrap_or(0))
        else {
            continue;
        };
        let column = indexed.point.column.0;
        if row >= size.rows || column >= size.columns {
            continue;
        }
        let cell = indexed.cell;
        let flags = cell.flags;
        let mut foreground = resolve(cell.fg);
        let mut back = resolve(cell.bg);
        if flags.contains(Flags::DIM) {
            foreground = Paint::solid(dim(&palette, cell.fg, foreground.rgb, background.rgb))
                .with_opacity(foreground.opacity);
        }
        if flags.contains(Flags::INVERSE) {
            std::mem::swap(&mut foreground, &mut back);
        }
        if selection.is_some_and(|range| range.contains(indexed.point)) {
            back = Paint::solid(palette.selection).with_opacity(palette.opacity.selection);
            // The theme's selected-text colour keeps selected text readable, as in
            // Alacritty (fix-agent-clipboard-copy).
            if let Some(text) = palette.selection_text {
                foreground = Paint::solid(text);
            }
        }
        if flags.contains(Flags::HIDDEN) {
            foreground = back;
        }
        rows[row][column] = StyledCell {
            ch: cell.c,
            zerowidth: cell.zerowidth().map(<[char]>::to_vec).unwrap_or_default(),
            background: back,
            style: TextStyle {
                color: foreground,
                bold: flags.contains(Flags::BOLD),
                italic: flags.contains(Flags::ITALIC),
                underline: underline(flags),
                strike: flags.contains(Flags::STRIKEOUT),
            },
            wide: flags.contains(Flags::WIDE_CHAR),
            spacer: flags.intersects(Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER),
        };
    }

    let cursor = content.cursor;
    let visible = cursor.shape != CursorShape::Hidden
        && content.mode.contains(TermMode::SHOW_CURSOR)
        && offset == 0;
    let cursor = visible
        .then(|| {
            let row = usize::try_from(cursor.point.line.0).ok()?;
            (row < size.rows && cursor.point.column.0 < size.columns).then_some(Cursor {
                row,
                column: cursor.point.column.0,
                shape: cursor.shape,
            })
        })
        .flatten();

    Frame {
        columns: size.columns,
        rows,
        cursor,
        background,
        cursor_color: resolve(Color::Named(NamedColor::Cursor)),
    }
}

fn underline(flags: Flags) -> Underline {
    if flags.contains(Flags::UNDERCURL) {
        Underline::Curly
    } else if flags.contains(Flags::DOUBLE_UNDERLINE) {
        Underline::Double
    } else if flags.contains(Flags::DOTTED_UNDERLINE) {
        Underline::Dotted
    } else if flags.contains(Flags::DASHED_UNDERLINE) {
        Underline::Dashed
    } else if flags.contains(Flags::UNDERLINE) {
        Underline::Single
    } else {
        Underline::None
    }
}

/// Faint text: the palette's dim colour for ANSI 0–7 when it has one, otherwise the
/// colour blended two thirds of the way from the background.
fn dim(palette: &Palette, original: Color, color: Rgb, background: Rgb) -> Rgb {
    if let Color::Named(named) = original
        && let Some(Some(dim)) = palette.dim.get(named as usize)
    {
        return *dim;
    }
    let blend = |front: u8, back: u8| -> u8 {
        let mixed = (u16::from(front) * 2 + u16::from(back)) / 3;
        u8::try_from(mixed).unwrap_or(u8::MAX)
    };
    Rgb {
        r: blend(color.r, background.r),
        g: blend(color.g, background.g),
        b: blend(color.b, background.b),
    }
}

/// Backgrounds that differ from the frame's own, merged across equal neighbours.
pub fn background_runs(frame: &Frame) -> Vec<BackgroundRun> {
    let mut runs = Vec::new();
    for (row, cells) in frame.rows.iter().enumerate() {
        let mut current: Option<BackgroundRun> = None;
        for (column, cell) in cells.iter().enumerate() {
            let color = cell.background;
            match current.as_mut() {
                Some(run) if run.color == color => run.cells += 1,
                _ => {
                    if let Some(done) = current.take() {
                        runs.push(done);
                    }
                    if color != frame.background {
                        current = Some(BackgroundRun {
                            row,
                            column,
                            cells: 1,
                            color,
                        });
                    }
                }
            }
        }
        runs.extend(current);
    }
    runs
}

/// Text runs for one row: equal styles merge, wide characters stand alone, spacers and
/// trailing blanks are skipped.
pub fn text_runs(row: usize, cells: &[StyledCell]) -> Vec<TextRun> {
    let last = cells
        .iter()
        .rposition(|cell| {
            !cell.spacer
                && (cell.ch != ' ' || cell.style.underline != Underline::None || cell.style.strike)
        })
        .map_or(0, |index| index + 1);
    let mut runs: Vec<TextRun> = Vec::new();
    for (column, cell) in cells.iter().enumerate().take(last) {
        if cell.spacer {
            continue;
        }
        let mut text = String::new();
        text.push(cell.ch);
        text.extend(&cell.zerowidth);
        let joins = !cell.wide
            && runs.last().is_some_and(|run| {
                !run.wide && run.style == cell.style && run.column + run.cells == column
            });
        if joins {
            let run = runs.last_mut().expect("checked above");
            run.text.push_str(&text);
            run.cells += 1;
        } else {
            runs.push(TextRun {
                row,
                column,
                text,
                cells: if cell.wide { 2 } else { 1 },
                style: cell.style,
                wide: cell.wide,
            });
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::model::GridSize;
    use crate::theme::Theme;

    fn frame_of(bytes: &[u8]) -> (Frame, Palette) {
        let palette = Theme::light().palette;
        let mut model = TerminalModel::new(
            GridSize {
                columns: 12,
                rows: 3,
            },
            palette.clone(),
        );
        model.feed(bytes);
        (build(&model), palette)
    }

    #[test]
    fn palette_colours_carry_their_opacity() {
        let mut palette = Theme::light().palette;
        palette.opacity.ansi[1] = 40;
        palette.opacity.selection = 70;
        let mut model = TerminalModel::new(
            GridSize {
                columns: 12,
                rows: 3,
            },
            palette,
        );
        model.feed(b"\x1b[31mr\x1b[38;2;1;2;3mt\x1b[0md");
        let frame = build(&model);
        let row = &frame.rows[0];
        assert_eq!(row[0].style.color.opacity, 40, "palette red at its opacity");
        assert_eq!(row[1].style.color.opacity, 100, "an exact colour is opaque");
        assert_eq!(row[2].style.color.opacity, 100);
    }

    #[test]
    fn colours_resolve_from_palette_256_and_truecolor() {
        let (frame, palette) = frame_of(b"\x1b[31mr\x1b[38;5;196mx\x1b[38;2;1;2;3mt\x1b[0md");
        let row = &frame.rows[0];
        assert_eq!(row[0].style.color.rgb, palette.ansi[1]);
        assert_eq!(row[1].style.color.rgb, Rgb { r: 255, g: 0, b: 0 });
        assert_eq!(row[2].style.color.rgb, Rgb { r: 1, g: 2, b: 3 });
        assert_eq!(row[3].style.color.rgb, palette.foreground);
    }

    #[test]
    fn inverse_bold_italic_and_underlines() {
        let (frame, palette) = frame_of(b"\x1b[7mi\x1b[0m\x1b[1;3mb\x1b[0m\x1b[4:3mc\x1b[0m");
        let row = &frame.rows[0];
        assert_eq!(row[0].background.rgb, palette.foreground);
        assert_eq!(row[0].style.color.rgb, palette.background);
        assert!(row[1].style.bold && row[1].style.italic);
        assert_eq!(row[2].style.underline, Underline::Curly);
    }

    #[test]
    fn wide_characters_take_two_cells_and_stand_alone() {
        let (frame, _) = frame_of("a界b".as_bytes());
        let row = &frame.rows[0];
        assert!(row[1].wide);
        assert!(row[2].spacer);
        assert_eq!(row[3].ch, 'b');
        let runs = text_runs(0, row);
        let shape: Vec<(usize, &str, usize)> = runs
            .iter()
            .map(|run| (run.column, run.text.as_str(), run.cells))
            .collect();
        assert_eq!(shape, [(0, "a", 1), (1, "界", 2), (3, "b", 1)]);
    }

    #[test]
    fn equal_styles_merge_and_trailing_blanks_are_dropped() {
        let (frame, _) = frame_of(b"ab \x1b[1mcd\x1b[0m   ");
        let runs = text_runs(0, &frame.rows[0]);
        let shape: Vec<(&str, bool)> = runs
            .iter()
            .map(|run| (run.text.as_str(), run.style.bold))
            .collect();
        assert_eq!(shape, [("ab ", false), ("cd", true)]);
    }

    #[test]
    fn backgrounds_merge_and_skip_the_default() {
        let (frame, palette) = frame_of(b"\x1b[44m   \x1b[0m  \x1b[41m \x1b[0m");
        let runs = background_runs(&frame);
        assert_eq!(runs.len(), 2);
        assert_eq!(
            (runs[0].column, runs[0].cells, runs[0].color.rgb),
            (0, 3, palette.ansi[4])
        );
        assert_eq!(
            (runs[1].column, runs[1].cells, runs[1].color.rgb),
            (5, 1, palette.ansi[1])
        );
    }

    #[test]
    fn cursor_follows_output_and_hides_when_asked() {
        let (frame, _) = frame_of(b"abc");
        assert_eq!(
            frame.cursor.map(|cursor| (cursor.row, cursor.column)),
            Some((0, 3))
        );
        let (hidden, _) = frame_of(b"abc\x1b[?25l");
        assert_eq!(hidden.cursor, None);
        let (beam, _) = frame_of(b"\x1b[6 q");
        assert_eq!(
            beam.cursor.map(|cursor| cursor.shape),
            Some(CursorShape::Beam)
        );
    }

    #[test]
    fn dim_text_is_fainter() {
        let (frame, palette) = frame_of(b"\x1b[2md");
        assert_ne!(frame.rows[0][0].style.color.rgb, palette.foreground);
        assert!(
            crate::theme::contrast(frame.rows[0][0].style.color.rgb, palette.background)
                < crate::theme::contrast(palette.foreground, palette.background)
        );
    }

    #[test]
    fn selection_paints_its_background() {
        let palette = Theme::light().palette;
        let mut model = TerminalModel::new(
            GridSize {
                columns: 12,
                rows: 3,
            },
            palette.clone(),
        );
        model.feed(b"select me");
        model.start_selection(
            0,
            0,
            alacritty_terminal::index::Side::Left,
            alacritty_terminal::selection::SelectionType::Simple,
        );
        model.extend_selection(0, 5, alacritty_terminal::index::Side::Right);
        let frame = build(&model);
        assert_eq!(frame.rows[0][2].background.rgb, palette.selection);
        assert_eq!(
            frame.rows[0][2].style.color.rgb, palette.foreground,
            "no selection text colour: the cell keeps its own"
        );
        assert_eq!(frame.rows[0][8].background.rgb, palette.background);

        let mut themed = palette.clone();
        themed.selection_text = Some(alacritty_terminal::vte::ansi::Rgb {
            r: 0x1E,
            g: 0x1E,
            b: 0x2E,
        });
        model.set_palette(themed.clone());
        let frame = build(&model);
        assert_eq!(
            frame.rows[0][2].style.color.rgb,
            themed.selection_text.unwrap(),
            "the theme's selected-text colour, as Catppuccin sets"
        );
        assert_eq!(
            frame.rows[0][8].style.color.rgb, themed.foreground,
            "outside it, unchanged"
        );
    }
}
