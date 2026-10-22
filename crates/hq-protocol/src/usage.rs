//! How usage readings are shown, shared by `hq usage` and the app's usage strip so
//! both say the same thing (`add-quota-footer` design D3).

use chrono::{DateTime, TimeZone};
use hq_signals::quota::{standing_windows, window_label};

use crate::{Account, AgentUsage, UsageWindow};

/// The strip names the agent nearest its limit from this share used.
pub const LABEL_FROM_PERCENT: f64 = 50.0;
/// From this share used the ring turns amber.
pub const HIGH_PERCENT: f64 = 90.0;
/// Readings follow activity and run at least every ten minutes, so one older than this
/// means updates are failing.
pub const OVERDUE_AFTER_SECS: u64 = 15 * 60;

/// What the limits belong to, in words: "Max plan", "API key", "Amazon Bedrock".
pub fn account_text(account: &Account) -> String {
    match account {
        Account::Subscription { plan: Some(plan) } if !plan.is_empty() => {
            let mut letters = plan.chars();
            let first = letters.next().map(|first| first.to_uppercase().to_string());
            format!("{}{} plan", first.unwrap_or_default(), letters.as_str())
        }
        Account::Subscription { .. } => "Subscription".to_owned(),
        Account::ApiKey => "API key".to_owned(),
        Account::Cloud { provider } => match provider.as_str() {
            "bedrock" => "Amazon Bedrock".to_owned(),
            "vertex" => "Google Vertex AI".to_owned(),
            "foundry" => "Microsoft Foundry".to_owned(),
            other => other.to_owned(),
        },
    }
}

/// Whether the window has reset since the reading, so its number no longer holds.
pub fn has_reset(window: &UsageWindow, read_at: u64, now_secs: u64) -> bool {
    window
        .resets_at
        .is_some_and(|reset| reset <= now_secs && read_at < reset)
}

/// The windows whose numbers still hold.
pub fn current_windows(usage: &AgentUsage, now_secs: u64) -> Vec<&UsageWindow> {
    let Some(read_at) = usage.read_at else {
        return Vec::new();
    };
    usage
        .windows
        .iter()
        .filter(|window| !has_reset(window, read_at, now_secs))
        .collect()
}

/// One window line of an agent's usage details.
#[derive(Debug, PartialEq)]
pub enum WindowRow<'a> {
    Reading(&'a UsageWindow),
    /// A window the agent always has but left out of the reading because it had
    /// reset; its length in minutes.
    ResetBefore(u64),
}

/// The reading's windows, with any standing window it left out, shortest first and
/// per-model windows after the all-models one of the same length.
pub fn window_rows(usage: &AgentUsage) -> Vec<WindowRow<'_>> {
    if usage.read_at.is_none() {
        return Vec::new();
    }
    let mut rows: Vec<(Option<u64>, bool, WindowRow<'_>)> = usage
        .windows
        .iter()
        .map(|window| {
            (
                window.minutes,
                window.model.is_some(),
                WindowRow::Reading(window),
            )
        })
        .collect();
    for &minutes in standing_windows(usage.agent) {
        if !usage
            .windows
            .iter()
            .any(|window| window.minutes == Some(minutes) && window.model.is_none())
        {
            rows.push((Some(minutes), false, WindowRow::ResetBefore(minutes)));
        }
    }
    rows.sort_by_key(|(minutes, per_model, _)| (minutes.unwrap_or(u64::MAX), *per_model));
    rows.into_iter().map(|(_, _, row)| row).collect()
}

/// The current window with the highest share used.
pub fn tightest(usage: &AgentUsage, now_secs: u64) -> Option<&UsageWindow> {
    current_windows(usage, now_secs)
        .into_iter()
        .max_by(|a, b| a.used_percent.total_cmp(&b.used_percent))
}

/// How far the ring is filled, 0 to 100; `None` draws a dotted ring.
pub fn fill(usage: &AgentUsage, now_secs: u64) -> Option<f64> {
    tightest(usage, now_secs).map(|window| window.used_percent.clamp(0.0, 100.0))
}

pub fn percent_text(percent: f64) -> String {
    format!("{}%", percent.round() as i64)
}

/// A window with a number that no longer holds.
pub const RESET_SINCE: &str = "reset since this reading";
/// A standing window the reading left out.
pub const RESET_BEFORE: &str = "reset before this reading";

/// "5h", "week", "week · Fable".
pub fn window_name(window: &UsageWindow) -> String {
    let length = window_label(window.minutes);
    match &window.model {
        Some(model) => format!("{length} · {model}"),
        None => length,
    }
}

/// "5h", "week".
pub fn length_name(minutes: u64) -> String {
    window_label(Some(minutes))
}

/// "codex 82% · 5h" for the agent nearest its limit, once one has used half a window.
pub fn label(agents: &[AgentUsage], now_secs: u64) -> Option<String> {
    agents
        .iter()
        .filter_map(|usage| tightest(usage, now_secs).map(|window| (usage, window)))
        .filter(|(_, window)| window.used_percent >= LABEL_FROM_PERCENT)
        .max_by(|(_, a), (_, b)| a.used_percent.total_cmp(&b.used_percent))
        .map(|(usage, window)| {
            format!(
                "{} {} · {}",
                usage.agent,
                percent_text(window.used_percent),
                window_name(window)
            )
        })
}

/// "Couldn't update since 3:12 PM" when the latest update failed or a reading is
/// overdue; `None` while readings are current.
pub fn update_note<Tz: TimeZone>(usage: &AgentUsage, now: &DateTime<Tz>) -> Option<String>
where
    Tz::Offset: std::fmt::Display,
{
    let read_at = usage.read_at?;
    let now_secs = u64::try_from(now.timestamp()).unwrap_or(0);
    let overdue = now_secs.saturating_sub(read_at) > OVERDUE_AFTER_SECS;
    if usage.problem.is_none() && !overdue {
        return None;
    }
    let zone = now.timezone();
    let at = zone
        .timestamp_opt(i64::try_from(read_at).ok()?, 0)
        .single()?;
    let when = if at.date_naive() == now.date_naive() {
        at.format("%-I:%M %p").to_string()
    } else {
        at.format("%a %-I:%M %p").to_string()
    };
    Some(format!("Couldn't update since {when}"))
}

/// "resets 3:40 PM" today, "resets Tue 9:00 AM" within the week, then the date.
pub fn reset_text<Tz: TimeZone>(resets_at: u64, now: &DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    let zone = now.timezone();
    let Some(at) = i64::try_from(resets_at)
        .ok()
        .and_then(|secs| zone.timestamp_opt(secs, 0).single())
    else {
        return "reset time unknown".to_owned();
    };
    let days = (at.date_naive() - now.date_naive()).num_days();
    let time = at.format("%-I:%M %p");
    match days {
        ..=0 => format!("resets {time}"),
        1..=6 => format!("resets {} {time}", at.format("%a")),
        _ => format!("resets {} {time}", at.format("%-d %b")),
    }
}

#[cfg(test)]
mod tests {
    use chrono::FixedOffset;
    use hq_signals::event::Agent;

    use super::*;

    const NOW: u64 = 1_000_000;

    fn window(minutes: u64, used: f64, resets_at: u64) -> UsageWindow {
        UsageWindow {
            minutes: Some(minutes),
            used_percent: used,
            resets_at: Some(resets_at),
            model: None,
        }
    }

    fn usage(agent: Agent, windows: Vec<UsageWindow>, read_at: Option<u64>) -> AgentUsage {
        AgentUsage {
            agent,
            windows,
            read_at,
            problem: None,
            account: None,
        }
    }

    #[test]
    fn accounts_in_words() {
        let plan = |plan: Option<&str>| {
            account_text(&Account::Subscription {
                plan: plan.map(str::to_owned),
            })
        };
        assert_eq!(plan(Some("max")), "Max plan");
        assert_eq!(plan(Some("plus")), "Plus plan");
        assert_eq!(plan(None), "Subscription");
        assert_eq!(account_text(&Account::ApiKey), "API key");
        assert_eq!(
            account_text(&Account::Cloud {
                provider: "vertex".into()
            }),
            "Google Vertex AI"
        );
    }

    #[test]
    fn fill_uses_the_tightest_window_that_still_holds() {
        let codex = usage(
            Agent::Codex,
            vec![
                window(300, 82.4, NOW + 600),
                window(10_080, 20.0, NOW + 86_400),
            ],
            Some(NOW - 60),
        );
        assert_eq!(fill(&codex, NOW), Some(82.4));
        let claude = usage(
            Agent::Claude,
            vec![
                window(300, 95.0, NOW - 3_600),
                window(10_080, 30.0, NOW + 86_400),
            ],
            Some(NOW - 3 * 3_600),
        );
        assert_eq!(
            fill(&claude, NOW),
            Some(30.0),
            "the five-hour window reset since the reading"
        );
        assert!(has_reset(&claude.windows[0], NOW - 3 * 3_600, NOW));
        assert_eq!(fill(&usage(Agent::Agy, vec![], None), NOW), None);
    }

    #[test]
    fn a_window_claude_left_out_is_shown_as_reset() {
        let week = window(10_080, 22.0, NOW + 86_400);
        let claude = usage(Agent::Claude, vec![week.clone()], Some(NOW - 60));
        assert_eq!(
            window_rows(&claude),
            vec![WindowRow::ResetBefore(300), WindowRow::Reading(&week)]
        );
        assert_eq!(length_name(300), "5h");
        assert_eq!(window_name(&week), "week");
        let codex = usage(Agent::Codex, vec![week.clone()], Some(NOW - 60));
        assert_eq!(window_rows(&codex), vec![WindowRow::Reading(&week)]);
        assert_eq!(window_rows(&usage(Agent::Claude, vec![], None)), vec![]);
    }

    #[test]
    fn label_names_the_agent_nearest_its_limit_from_half() {
        let agents = [
            usage(Agent::Claude, vec![window(300, 60.0, NOW + 10)], Some(NOW)),
            usage(Agent::Codex, vec![window(300, 82.4, NOW + 10)], Some(NOW)),
            usage(Agent::Agy, vec![], None),
        ];
        assert_eq!(label(&agents, NOW).as_deref(), Some("codex 82% · 5h"));
        let calm = [usage(
            Agent::Codex,
            vec![window(10_080, 49.0, NOW + 10)],
            Some(NOW),
        )];
        assert_eq!(label(&calm, NOW), None);
    }

    #[test]
    fn a_note_only_when_an_update_failed_or_is_overdue() {
        // Tuesday 20 October 2026, 10:00 at UTC+7.
        let zone = FixedOffset::east_opt(7 * 3_600).unwrap();
        let now = zone.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap();
        let secs = u64::try_from(now.timestamp()).unwrap();
        let mut codex = usage(Agent::Codex, vec![], Some(secs - 240));
        assert_eq!(update_note(&codex, &now), None);
        codex.problem = Some("timed out".into());
        assert_eq!(
            update_note(&codex, &now).as_deref(),
            Some("Couldn't update since 9:56 AM")
        );
        codex.problem = None;
        codex.read_at = Some(secs - 26 * 3_600);
        assert_eq!(
            update_note(&codex, &now).as_deref(),
            Some("Couldn't update since Mon 8:00 AM")
        );
        assert_eq!(update_note(&usage(Agent::Agy, vec![], None), &now), None);
    }

    #[test]
    fn per_model_windows_follow_the_all_models_one() {
        let mut fable = window(10_080, 3.0, NOW + 86_400);
        fable.model = Some("Fable".into());
        let week = window(10_080, 26.0, NOW + 86_400);
        let session = window(300, 15.0, NOW + 600);
        let claude = usage(
            Agent::Claude,
            vec![fable.clone(), week.clone(), session.clone()],
            Some(NOW),
        );
        assert_eq!(
            window_rows(&claude),
            vec![
                WindowRow::Reading(&session),
                WindowRow::Reading(&week),
                WindowRow::Reading(&fable)
            ]
        );
        assert_eq!(window_name(&fable), "week · Fable");
    }

    #[test]
    fn reset_times_in_local_twelve_hour_time() {
        // Tuesday 20 October 2026, 10:00 at UTC+7.
        let zone = FixedOffset::east_opt(7 * 3_600).unwrap();
        let now = zone.with_ymd_and_hms(2026, 10, 20, 10, 0, 0).unwrap();
        let at = |d, h, m| {
            u64::try_from(
                zone.with_ymd_and_hms(2026, 10, d, h, m, 0)
                    .unwrap()
                    .timestamp(),
            )
            .unwrap()
        };
        assert_eq!(reset_text(at(20, 15, 40), &now), "resets 3:40 PM");
        assert_eq!(reset_text(at(23, 9, 0), &now), "resets Fri 9:00 AM");
        assert_eq!(reset_text(at(28, 9, 5), &now), "resets 28 Oct 9:05 AM");
    }
}
