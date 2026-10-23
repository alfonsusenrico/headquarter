//! What a copy from the pane puts on the clipboard: the selected text, tidied as Claude
//! Code tidies its own copies (fix-agent-clipboard-copy D2).

/// Removes trailing spaces from every line, the margin every line shares, and blank
/// lines at both ends; blank lines inside are kept. When the selection started mid-row,
/// its first line has no margin to remove and does not count towards the shared one.
pub fn tidy(text: &str, first_line_partial: bool) -> String {
    let lines: Vec<&str> = text.lines().map(str::trim_end).collect();
    let Some(start) = lines.iter().position(|line| !line.is_empty()) else {
        return String::new();
    };
    let end = lines
        .iter()
        .rposition(|line| !line.is_empty())
        .unwrap_or(start);
    let body = &lines[start..=end];
    // Only the selection's own first line can be partial.
    let partial = first_line_partial && start == 0;
    let margin = body
        .iter()
        .enumerate()
        .filter(|(index, line)| !line.is_empty() && !(partial && *index == 0))
        .map(|(_, line)| line.len() - line.trim_start_matches(' ').len())
        .min()
        .unwrap_or(0);
    body.iter()
        .enumerate()
        .map(|(index, line)| {
            if partial && index == 0 {
                line.trim_start_matches(' ')
            } else if line.is_empty() {
                line
            } else {
                &line[margin..]
            }
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

/// The note shown after a copy.
pub fn copied_note(text: &str) -> String {
    match text.lines().count() {
        0 | 1 => "Copied".to_owned(),
        lines => format!("Copied {lines} lines"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn margins_trailing_spaces_and_blank_edges_go() {
        let selected = "\n  SELECT name,   \n    COUNT(*)\n\n  FROM school   \n\n";
        assert_eq!(
            tidy(selected, false),
            "SELECT name,\n  COUNT(*)\n\nFROM school",
            "the shared two-space margin goes, inner indentation and the blank line stay"
        );
    }

    #[test]
    fn a_selection_started_mid_row_keeps_the_others_margin_right() {
        // Started on "school_city," in a reply indented by two spaces.
        let selected = "school_city,\n      CONVERT(name)\n  FROM prospect";
        assert_eq!(
            tidy(selected, true),
            "school_city,\n    CONVERT(name)\nFROM prospect"
        );
        assert_eq!(
            tidy(selected, false),
            "school_city,\n      CONVERT(name)\n  FROM prospect",
            "a full first line with no margin means there is none to remove"
        );
    }

    #[test]
    fn nothing_but_spaces_copies_nothing() {
        assert_eq!(tidy("   \n  \n", false), "");
    }

    #[test]
    fn the_note_counts_lines() {
        assert_eq!(copied_note("one"), "Copied");
        assert_eq!(copied_note("a\nb\nc"), "Copied 3 lines");
    }
}
