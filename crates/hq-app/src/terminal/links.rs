//! Links in the pane (add-terminal-links): a link an agent marked up (OSC 8), or a
//! plain `http(s)` address in the text, across the lines the terminal wrapped. Only web
//! addresses open, so a marked-up `file:` or app link cannot launch a program.

use std::ops::RangeInclusive;

use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::Term;
use alacritty_terminal::term::cell::{Cell, Hyperlink};
use alacritty_terminal::term::search::RegexSearch;

/// Alacritty's URL hint, for web addresses only: everything up to a space, a control
/// character, or a character that rarely belongs to an address.
const WEB_ADDRESS: &str = "https?://[^\u{0000}-\u{001F}\u{007F}-\u{009F}<>\"\\s{-}\\^⟨⟩`\\\\]+";

/// A link in the grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Link {
    /// Marked up by the agent: every cell carrying its address, wherever it wrapped;
    /// agents may close and reopen a link on each row they wrap it over.
    Marked(Hyperlink),
    /// Found in the text: the address and the cells it spans.
    Plain {
        address: String,
        cells: RangeInclusive<Point>,
    },
}

impl Link {
    pub fn address(&self) -> &str {
        match self {
            Link::Marked(hyperlink) => hyperlink.uri(),
            Link::Plain { address, .. } => address,
        }
    }

    /// Whether `cell`, at `point`, is part of this link.
    pub fn covers(&self, point: Point, cell: &Cell) -> bool {
        match self {
            Link::Marked(hyperlink) => cell
                .hyperlink()
                .is_some_and(|other| other.uri() == hyperlink.uri()),
            Link::Plain { cells, .. } => cells.contains(&point),
        }
    }
}

pub struct LinkFinder {
    web_address: RegexSearch,
}

impl LinkFinder {
    pub fn new() -> LinkFinder {
        LinkFinder {
            web_address: RegexSearch::new(WEB_ADDRESS).expect("the web address pattern compiles"),
        }
    }

    /// The link at a grid point, if it is a web address.
    pub fn at<T>(&mut self, term: &Term<T>, point: Point) -> Option<Link> {
        if point.line < term.topmost_line()
            || point.line > term.bottommost_line()
            || point.column.0 >= term.columns()
        {
            return None;
        }
        if let Some(hyperlink) = term.grid()[point].hyperlink() {
            return opens(hyperlink.uri()).then_some(Link::Marked(hyperlink));
        }
        let end = term.line_search_right(point);
        let mut from = term.line_search_left(point);
        while let Some(found) = term.regex_search_right(&mut self.web_address, from, end) {
            let (start, last) = (*found.start(), *found.end());
            if point < start {
                return None;
            }
            if point <= last {
                let text = term.bounds_to_string(start, last);
                let address = trim_address(&text);
                let trimmed = text.chars().count() - address.chars().count();
                let last = step_back(term, last, trimmed);
                return (point <= last && opens(address)).then(|| Link::Plain {
                    address: address.to_owned(),
                    cells: start..=last,
                });
            }
            from = step_forward(term, last);
            if from <= last {
                return None;
            }
        }
        None
    }
}

/// Only web addresses open.
fn opens(address: &str) -> bool {
    let lower = address.to_ascii_lowercase();
    let rest = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"));
    rest.is_some_and(|rest| !rest.is_empty())
}

/// Drops what usually ends the sentence rather than the address: trailing punctuation,
/// quotes, and closing brackets without an opening one inside the address.
fn trim_address(text: &str) -> &str {
    let mut address = text;
    loop {
        let Some(last) = address.chars().last() else {
            return address;
        };
        let unmatched = |open: char| address.matches(open).count() < address.matches(last).count();
        let drop = match last {
            '.' | ',' | ':' | ';' | '!' | '?' | '\'' | '"' => true,
            ')' => unmatched('('),
            ']' => unmatched('['),
            _ => false,
        };
        if !drop {
            return address;
        }
        address = &address[..address.len() - last.len_utf8()];
    }
}

fn step_forward<T>(term: &Term<T>, point: Point) -> Point {
    if point.column.0 + 1 < term.columns() {
        Point::new(point.line, point.column + 1)
    } else {
        Point::new(point.line + 1, Column(0))
    }
}

fn step_back<T>(term: &Term<T>, mut point: Point, cells: usize) -> Point {
    for _ in 0..cells {
        point = if point.column.0 > 0 {
            Point::new(point.line, point.column - 1)
        } else {
            Point::new(point.line - 1i32, Column(term.columns() - 1))
        };
    }
    point
}

/// The grid point of a visible cell (row 0 is the top of the view).
pub fn view_point<T>(term: &Term<T>, row: usize, column: usize) -> Point {
    let offset = i32::try_from(term.grid().display_offset()).unwrap_or(0);
    Point::new(
        Line(i32::try_from(row).unwrap_or(0) - offset),
        Column(column),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::model::{GridSize, TerminalModel};
    use crate::theme::Theme;

    fn model(columns: usize, output: &[u8]) -> TerminalModel {
        let mut model = TerminalModel::new(GridSize { columns, rows: 6 }, Theme::light().palette);
        model.feed(output);
        model
    }

    fn address_at(model: &TerminalModel, row: usize, column: usize) -> Option<String> {
        let term = model.term();
        LinkFinder::new()
            .at(term, view_point(term, row, column))
            .map(|link| link.address().to_owned())
    }

    #[test]
    fn a_plain_address_is_found_from_any_of_its_cells_and_not_beside_it() {
        let model = model(40, b"see https://example.com/a?b=1 now");
        assert_eq!(
            address_at(&model, 0, 4).as_deref(),
            Some("https://example.com/a?b=1")
        );
        assert_eq!(
            address_at(&model, 0, 28).as_deref(),
            Some("https://example.com/a?b=1")
        );
        assert_eq!(address_at(&model, 0, 2), None);
        assert_eq!(address_at(&model, 0, 30), None);
    }

    #[test]
    fn an_address_the_terminal_wrapped_is_one_link() {
        let model = model(20, b"go https://example.com/long/path ok");
        let whole = Some("https://example.com/long/path".to_owned());
        assert_eq!(address_at(&model, 0, 10), whole);
        assert_eq!(address_at(&model, 1, 3), whole);
    }

    #[test]
    fn sentence_punctuation_and_unmatched_brackets_stay_out() {
        let model = model(
            80,
            b"(see https://example.com/wiki/A_(b)). And https://x.io/p, then",
        );
        assert_eq!(
            address_at(&model, 0, 10).as_deref(),
            Some("https://example.com/wiki/A_(b)")
        );
        assert_eq!(address_at(&model, 0, 35), None, "the closing bracket");
        assert_eq!(address_at(&model, 0, 45).as_deref(), Some("https://x.io/p"));
    }

    #[test]
    fn a_marked_up_link_opens_its_full_address_from_every_row() {
        let model = model(
            20,
            b"\x1b]8;;https://example.com/full/address\x07docs\r\n  page\x1b]8;;\x07 end",
        );
        let whole = Some("https://example.com/full/address".to_owned());
        assert_eq!(address_at(&model, 0, 1), whole);
        assert_eq!(address_at(&model, 1, 3), whole);
        assert_eq!(address_at(&model, 1, 8), None);
    }

    #[test]
    fn only_web_addresses_open() {
        let model = model(
            40,
            b"\x1b]8;;file:///Applications/Calculator.app\x07calc\x1b]8;;\x07 ftp://x.io",
        );
        assert_eq!(address_at(&model, 0, 1), None);
        assert_eq!(address_at(&model, 0, 7), None);
    }

    #[test]
    fn trimming() {
        assert_eq!(trim_address("https://a.io/x)."), "https://a.io/x");
        assert_eq!(trim_address("https://a.io/(x)"), "https://a.io/(x)");
        assert_eq!(trim_address("https://a.io/\"'"), "https://a.io/");
    }
}
