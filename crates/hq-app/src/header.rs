//! What the session header shows and what fits (`openspec/changes/add-session-header/
//! design.md`, D1, D3): each agent's details, missing ones left out, and the least
//! important dropped first when the pane is narrow. Pure, so it is tested without a
//! window.

use hq_signals::details::SessionDetails;

/// About how wide one character of the header's 12.5 px details is.
const CHAR_PX: f32 = 7.0;
/// About how wide one character of the 14 px semibold title is.
const TITLE_CHAR_PX: f32 = 8.2;
/// A separator with the gaps on both sides of it (refresh-pane-and-status D3).
const SEPARATOR_PX: f32 = 26.0;
/// The agent mark, the side padding and the gap between the two halves.
const FIXED_PX: f32 = 20.0 + 32.0 + 16.0;
/// The context meter's bar beside its words.
const METER_PX: f32 = 66.0;
/// Folders are never shortened below this many characters.
const MIN_FOLDER_CHARS: usize = 12;

#[derive(Debug, Clone, PartialEq)]
pub struct HeaderLine {
    pub title: String,
    /// The full folder, `~` for home, shortened in the middle when it does not fit.
    pub folder: String,
    pub branch: Option<String>,
    pub model: Option<String>,
    /// Share of the context window used, 0 to 100.
    pub used: Option<f64>,
    /// "38% left".
    pub left: Option<String>,
    /// "$0.42".
    pub cost: Option<String>,
}

impl HeaderLine {
    pub fn new(title: &str, folder: &str, details: &SessionDetails) -> HeaderLine {
        HeaderLine {
            title: title.split_whitespace().collect::<Vec<_>>().join(" "),
            folder: folder.to_owned(),
            branch: details.branch.clone(),
            model: details.model.clone(),
            used: details.context_used_percent,
            left: details
                .context_left_percent
                .map(|left| format!("{} left", percent(left))),
            cost: details.cost_usd.map(dollars),
        }
    }

    /// The line as it fits in `width` pixels: cost, then "left", the model and the
    /// branch are dropped in that order, then the folder is shortened in the middle,
    /// and only then the title at its end (the element truncates it).
    pub fn fitted(mut self, width: f32) -> HeaderLine {
        let drops: [fn(&mut HeaderLine) -> bool; 4] = [
            |line| line.cost.take().is_some(),
            |line| line.left.take().is_some(),
            |line| line.model.take().is_some(),
            |line| line.branch.take().is_some(),
        ];
        for drop in drops {
            if self.width() <= width {
                return self;
            }
            drop(&mut self);
        }
        let over = self.width() - width;
        if over > 0.0 {
            let chars = self.folder.chars().count();
            let keep = chars
                .saturating_sub((over / CHAR_PX).ceil() as usize)
                .max(MIN_FOLDER_CHARS);
            self.folder = middle(&self.folder, keep);
        }
        self
    }

    fn width(&self) -> f32 {
        let text = |text: &str| text.chars().count() as f32 * CHAR_PX;
        let item = |value: &Option<String>| {
            value
                .as_deref()
                .map_or(0.0, |value| text(value) + SEPARATOR_PX)
        };
        let title = self.title.chars().count() as f32 * TITLE_CHAR_PX;
        let mut total = FIXED_PX + title + SEPARATOR_PX + text(&self.folder);
        total += item(&self.branch.as_ref().map(|branch| format!("⎇ {branch}")));
        total += item(&self.model);
        total += self.used.map_or(0.0, |used| {
            METER_PX + text(&context_words(used)) + SEPARATOR_PX
        });
        total += item(&self.left);
        total += item(&self.cost);
        total
    }
}

/// "62% context".
pub fn context_words(used: f64) -> String {
    format!("{} context", percent(used))
}

fn percent(value: f64) -> String {
    format!("{}%", value.round() as i64)
}

fn dollars(cost: f64) -> String {
    if cost > 0.0 && cost < 0.01 {
        "<$0.01".to_owned()
    } else {
        format!("${cost:.2}")
    }
}

/// `text` cut to `keep` characters with an ellipsis in the middle, so both the start
/// and the end of a path stay readable.
pub fn middle(text: &str, keep: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= keep || keep < 3 {
        return text.to_owned();
    }
    let tail = (keep - 1) / 2;
    let head = keep - 1 - tail;
    let mut out: String = chars[..head].iter().collect();
    out.push('…');
    out.extend(&chars[chars.len() - tail..]);
    out
}

/// Token counts with thousands separators: "15,234".
pub fn tokens(count: u64) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

#[cfg(test)]
mod tests {
    use hq_signals::details::TokenCounts;

    use super::*;

    fn claude() -> SessionDetails {
        SessionDetails {
            model: Some("Opus".into()),
            context_used_percent: Some(61.6),
            context_left_percent: Some(38.4),
            context_window: Some(200_000),
            tokens: Some(TokenCounts::default()),
            cost_usd: Some(0.4237),
            branch: Some("main".into()),
        }
    }

    #[test]
    fn everything_fits_on_a_wide_pane() {
        let line =
            HeaderLine::new("Fix the\nflaky login test", "~/work/api", &claude()).fitted(1400.0);
        assert_eq!(line.title, "Fix the flaky login test");
        assert_eq!(line.left.as_deref(), Some("38% left"));
        assert_eq!(line.cost.as_deref(), Some("$0.42"));
        assert_eq!(context_words(line.used.unwrap()), "62% context");
        assert_eq!(line.branch.as_deref(), Some("main"));
    }

    #[test]
    fn missing_details_are_left_out() {
        let agy = SessionDetails {
            branch: Some("main".into()),
            ..SessionDetails::default()
        };
        let line = HeaderLine::new("notes", "~/notes", &agy);
        assert_eq!(
            (line.model, line.used, line.left, line.cost),
            (None, None, None, None)
        );
    }

    #[test]
    fn narrow_panes_drop_in_order_then_shorten_the_folder() {
        let full = HeaderLine::new(
            "Fix the flaky login test",
            "~/work/projects/acme/api",
            &claude(),
        );
        let wide = full.width();
        let step = |dropped: f32| full.clone().fitted(wide - dropped);
        let no_cost = step(1.0);
        assert_eq!(no_cost.cost, None);
        assert!(no_cost.left.is_some());
        let no_left = step(80.0);
        assert_eq!((no_left.cost.clone(), no_left.left.clone()), (None, None));
        assert!(no_left.model.is_some());
        let tight = full.clone().fitted(520.0);
        assert_eq!((tight.model.clone(), tight.branch.clone()), (None, None));
        assert!(tight.used.is_some(), "the meter stays");
        assert!(tight.folder.contains('…'), "{}", tight.folder);
        assert!(tight.folder.starts_with("~/w") && tight.folder.ends_with("api"));
        assert_eq!(
            tight.title, "Fix the flaky login test",
            "the title keeps its words"
        );
    }

    #[test]
    fn helpers() {
        assert_eq!(middle("~/work/projects/acme/api", 12), "~/work…e/api");
        assert_eq!(middle("short", 12), "short");
        assert_eq!(tokens(15_234), "15,234");
        assert_eq!(tokens(999), "999");
        assert_eq!(tokens(1_000_000), "1,000,000");
        assert_eq!(dollars(0.004), "<$0.01");
        assert_eq!(dollars(0.0), "$0.00");
        assert_eq!(dollars(12.5), "$12.50");
    }
}
