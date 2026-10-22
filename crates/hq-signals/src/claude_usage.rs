//! Claude Code's plan limits as its own `/usage` screen shows them, read from the
//! screen's text (`refresh-usage-on-activity` design D2). The layout is an unstable
//! external contract: blocks are found by their headings, unknown lines are skipped, and
//! a screen without the session and all-models week blocks is an error rather than a
//! guess. Observed with Claude Code 2.1.304 to 2.1.306.
//!
//! ```text
//! Current session
//! ███████▌                     15%used
//! Resets 11:30am (Asia/Jakarta)
//! Current week (all models)
//! █████████████                26%used
//! Resets Oct 24 at 3pm (Asia/Jakarta)
//! Current week (Fable)
//! ...
//! ```

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, TimeZone};

use crate::quota::QuotaWindow;

const FIVE_HOURS_MINS: u64 = 5 * 60;
const WEEK_MINS: u64 = 7 * 24 * 60;
/// How many lines below a heading its percentage and reset time are looked for.
const BLOCK_LINES: usize = 4;

/// The windows on a `/usage` screen. Reset times are read as the local time of `now`,
/// which is the zone Claude Code prints them in.
pub fn parse_usage_screen<Tz: TimeZone>(
    lines: &[String],
    now: &DateTime<Tz>,
) -> Result<Vec<QuotaWindow>, String> {
    let mut windows = Vec::new();
    for (index, line) in lines.iter().enumerate() {
        let Some((minutes, model)) = heading(line.trim()) else {
            continue;
        };
        // A block ends at the next heading, so one never takes another's number.
        let block: Vec<&String> = lines[index + 1..]
            .iter()
            .take(BLOCK_LINES)
            .take_while(|line| heading(line.trim()).is_none())
            .collect();
        let Some(used_percent) = block.iter().find_map(|line| percent_used(line)) else {
            continue;
        };
        let resets_at = block
            .iter()
            .find_map(|line| line.trim().strip_prefix("Resets "))
            .and_then(|text| reset_time(text, now));
        windows.push(QuotaWindow {
            duration_mins: Some(minutes),
            used_percent,
            resets_at,
            model,
        });
    }
    let has = |minutes: u64| {
        windows
            .iter()
            .any(|window| window.duration_mins == Some(minutes) && window.model.is_none())
    };
    if !has(FIVE_HOURS_MINS) || !has(WEEK_MINS) {
        return Err("Claude Code's /usage screen did not show its limits".to_owned());
    }
    Ok(windows)
}

/// A block heading: the window's length, and its model for a per-model window.
fn heading(line: &str) -> Option<(u64, Option<String>)> {
    if line == "Current session" {
        return Some((FIVE_HOURS_MINS, None));
    }
    let inside = line
        .strip_prefix("Current week (")?
        .strip_suffix(')')?
        .trim();
    if inside.is_empty() {
        return None;
    }
    let model = (inside != "all models").then(|| inside.to_owned());
    Some((WEEK_MINS, model))
}

/// `15` from a bar line ending in "15%used" or "15% used".
fn percent_used(line: &str) -> Option<f64> {
    let at = line.find('%')?;
    if !line[at + 1..].trim_start().starts_with("used") {
        return None;
    }
    let digits: String = line[..at]
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    digits.parse().ok()
}

/// Unix seconds for "11:30am (Asia/Jakarta)" or "Oct 24 at 3pm (Asia/Jakarta)": the
/// next such time from `now`.
fn reset_time<Tz: TimeZone>(text: &str, now: &DateTime<Tz>) -> Option<u64> {
    let text = text.split(" (").next()?.trim();
    let (date, time) = match text.split_once(" at ") {
        Some((date, time)) => (Some(date.trim()), time.trim()),
        None => (None, text),
    };
    let time = clock(time)?;
    let zone = now.timezone();
    let local = |day: NaiveDate| zone.from_local_datetime(&day.and_time(time)).earliest();
    let today = now.date_naive();
    let at = match date {
        None => {
            let at = local(today)?;
            if at < *now {
                local(today + Duration::days(1))?
            } else {
                at
            }
        }
        Some(date) => {
            let (month, day) = date.split_once(' ')?;
            let month = month_number(month)?;
            let day: u32 = day.trim().parse().ok()?;
            let at = local(NaiveDate::from_ymd_opt(today.year(), month, day)?)?;
            // "Jan 2" read in late December belongs to the next year.
            if at < now.clone() - Duration::days(1) {
                local(NaiveDate::from_ymd_opt(today.year() + 1, month, day)?)?
            } else {
                at
            }
        }
    };
    u64::try_from(at.timestamp()).ok()
}

/// "11:30am", "3pm", "12am".
fn clock(text: &str) -> Option<NaiveTime> {
    let lower = text.to_ascii_lowercase();
    let (digits, afternoon) = if let Some(rest) = lower.strip_suffix("pm") {
        (rest, true)
    } else {
        (lower.strip_suffix("am")?, false)
    };
    let (hour, minute) = match digits.split_once(':') {
        Some((hour, minute)) => (hour.parse::<u32>().ok()?, minute.parse::<u32>().ok()?),
        None => (digits.parse::<u32>().ok()?, 0),
    };
    if !(1..=12).contains(&hour) {
        return None;
    }
    let hour = match (hour, afternoon) {
        (12, false) => 0,
        (12, true) => 12,
        (hour, true) => hour + 12,
        (hour, false) => hour,
    };
    NaiveTime::from_hms_opt(hour, minute, 0)
}

fn month_number(name: &str) -> Option<u32> {
    let months = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let name = name.get(..3)?.to_ascii_lowercase();
    months
        .iter()
        .position(|month| *month == name)
        .and_then(|index| u32::try_from(index + 1).ok())
}

#[cfg(test)]
mod tests {
    use chrono::FixedOffset;

    use super::*;

    /// The layout Claude Code 2.1.306 drew, with the owner's skills and costs replaced.
    const SCREEN: &str = "   Settings  Status   Config   Usage Stats
Session
Total cost:            $0.0000
Current session
███████▌                                          15%used
Resets 11:30am (Asia/Jakarta)
Current week (all models)
█████████████                                     26%used
Resets Oct 24 at 3pm (Asia/Jakarta)
Current week (Fable)
█▌                                                3%used
Resets Oct 24 at 3pm (Asia/Jakarta)
What's contributing to your limits usage?
97%  Placeholder advice line
Skills% of usage
/some-skill3%
Esc to cancel";

    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_owned).collect()
    }

    /// Wednesday 21 October 2026, 08:00 at UTC+7.
    fn now() -> DateTime<FixedOffset> {
        FixedOffset::east_opt(7 * 3_600)
            .unwrap()
            .with_ymd_and_hms(2026, 10, 21, 8, 0, 0)
            .unwrap()
    }

    fn at(day: u32, hour: u32, minute: u32) -> u64 {
        let zone = FixedOffset::east_opt(7 * 3_600).unwrap();
        u64::try_from(
            zone.with_ymd_and_hms(2026, 10, day, hour, minute, 0)
                .unwrap()
                .timestamp(),
        )
        .unwrap()
    }

    #[test]
    fn every_block_with_its_reset() {
        let windows = parse_usage_screen(&lines(SCREEN), &now()).unwrap();
        assert_eq!(
            windows,
            vec![
                QuotaWindow {
                    duration_mins: Some(300),
                    used_percent: 15.0,
                    resets_at: Some(at(21, 11, 30)),
                    model: None,
                },
                QuotaWindow {
                    duration_mins: Some(WEEK_MINS),
                    used_percent: 26.0,
                    resets_at: Some(at(24, 15, 0)),
                    model: None,
                },
                QuotaWindow {
                    duration_mins: Some(WEEK_MINS),
                    used_percent: 3.0,
                    resets_at: Some(at(24, 15, 0)),
                    model: Some("Fable".into()),
                },
            ],
            "the advice line's 97% is not a window"
        );
    }

    #[test]
    fn a_time_already_passed_today_is_tomorrow() {
        assert_eq!(
            reset_time("7:15am (Asia/Jakarta)", &now()),
            Some(at(22, 7, 15))
        );
        assert_eq!(reset_time("12am", &now()), Some(at(22, 0, 0)));
        assert_eq!(reset_time("12pm", &now()), Some(at(21, 12, 0)));
    }

    #[test]
    fn missing_blocks_or_garbage_are_errors() {
        let only_session = "Current session\n██ 15%used\nResets 11:30am";
        assert!(parse_usage_screen(&lines(only_session), &now()).is_err());
        assert!(parse_usage_screen(&lines("Loading…"), &now()).is_err());
        assert_eq!(reset_time("soon", &now()), None);
        assert_eq!(reset_time("Smarch 3 at 3pm", &now()), None);
    }

    #[test]
    fn a_block_without_a_reset_keeps_its_number() {
        let screen = "Current session\n██ 15% used\nCurrent week (all models)\n██ 26%used";
        let windows = parse_usage_screen(&lines(screen), &now()).unwrap();
        assert_eq!(windows[0].used_percent, 15.0);
        assert_eq!(windows[1].resets_at, None);
    }
}
