//! How usage readings are shown, shared by `hq usage` and the app's usage strip so
//! both say the same thing (`add-quota-footer` design D3).

use chrono::{DateTime, TimeZone};
use hq_signals::quota::{standing_windows, window_label};

use crate::{Account, AgentUsage, UsageWindow};

/// From this share used a window is near its limit: its ring and bar turn red and the
/// strip names it (owner, 2026-10-23).
pub const NEAR_LIMIT_PERCENT: f64 = 80.0;
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

/// The footer's two rings (owner, 2026-10-22): the inner one for the agent's shortest
/// all-models window that still holds (Claude Code's and Codex's 5 hours), the outer one
/// for its longest (the week). With one window, only the outer ring is filled; `None`
/// leaves a ring empty.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rings {
    pub inner: Option<f64>,
    pub outer: Option<f64>,
}

impl Rings {
    pub fn is_empty(&self) -> bool {
        self.inner.is_none() && self.outer.is_none()
    }
}

/// The windows the footer's rings show, shortest first: the all-models ones, or, for
/// an agent that reports only pools of models (agy), the pool nearest its limit, by the
/// same rule as the label (owner, 2026-10-23).
pub fn ring_windows(usage: &AgentUsage, now_secs: u64) -> Vec<&UsageWindow> {
    let current = current_windows(usage, now_secs);
    let mut windows: Vec<&UsageWindow> =
        if usage.windows.iter().any(|window| window.model.is_none()) {
            current
                .into_iter()
                .filter(|window| window.model.is_none())
                .collect()
        } else {
            let mut pools: Vec<&str> = Vec::new();
            for window in &current {
                if let Some(pool) = window.model.as_deref()
                    && !pools.contains(&pool)
                {
                    pools.push(pool);
                }
            }
            let nearest = |pool: &str| {
                current
                    .iter()
                    .filter(|window| window.model.as_deref() == Some(pool))
                    .map(|window| window.used_percent.round())
                    .fold(f64::MIN, f64::max)
            };
            // The first pool reported wins a tie.
            let chosen = pools.into_iter().reduce(|best, pool| {
                if nearest(pool) > nearest(best) {
                    pool
                } else {
                    best
                }
            });
            current
                .into_iter()
                .filter(|window| chosen.is_some() && window.model.as_deref() == chosen)
                .collect()
        };
    windows.sort_by_key(|window| window.minutes.unwrap_or(u64::MAX));
    windows
}

/// The fills of the footer's rings, 0 to 100.
pub fn rings(usage: &AgentUsage, now_secs: u64) -> Rings {
    let windows = ring_windows(usage, now_secs);
    let percent = |window: &UsageWindow| window.used_percent.clamp(0.0, 100.0);
    match windows.as_slice() {
        [] => Rings {
            inner: None,
            outer: None,
        },
        [only] => Rings {
            inner: None,
            outer: Some(percent(only)),
        },
        [shortest, .., longest] => Rings {
            inner: Some(percent(shortest)),
            outer: Some(percent(longest)),
        },
    }
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

/// The window nearest its limit as the owner reads it: the highest whole percentage,
/// and on a tie the shorter window, which resets sooner (owner, 2026-10-23).
pub fn nearest_limit(usage: &AgentUsage, now_secs: u64) -> Option<&UsageWindow> {
    let length = |window: &UsageWindow| window.minutes.unwrap_or(u64::MAX);
    current_windows(usage, now_secs).into_iter().max_by(|a, b| {
        a.used_percent
            .round()
            .total_cmp(&b.used_percent.round())
            .then_with(|| length(b).cmp(&length(a)))
    })
}

/// The strip's line once agents near their limits (owner, 2026-10-23): "codex 82% · 5h"
/// for a single agent, naming its nearest window, or "2 agents near their limit".
pub fn label(agents: &[AgentUsage], now_secs: u64) -> Option<String> {
    let near: Vec<(&AgentUsage, &UsageWindow)> = agents
        .iter()
        .filter_map(|usage| nearest_limit(usage, now_secs).map(|window| (usage, window)))
        .filter(|(_, window)| window.used_percent >= NEAR_LIMIT_PERCENT)
        .collect();
    match near.as_slice() {
        [] => None,
        [(usage, window)] => Some(format!(
            "{} {} · {}",
            usage.agent,
            percent_text(window.used_percent),
            window_name(window)
        )),
        several => Some(format!("{} agents near their limit", several.len())),
    }
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
    fn agy_rings_follow_the_pool_nearest_its_limit() {
        let pool = |minutes, used, name: &str| UsageWindow {
            model: Some(name.into()),
            ..window(minutes, used, NOW + 5_000)
        };
        let agy = usage(
            Agent::Agy,
            vec![
                pool(10_080, 11.0, "Gemini"),
                pool(300, 0.0, "Gemini"),
                pool(300, 85.0, "Claude and GPT"),
                pool(10_080, 40.0, "Claude and GPT"),
                pool(10_080, 0.0, "Gemini 4 Argon"),
            ],
            Some(NOW),
        );
        assert_eq!(
            rings(&agy, NOW),
            Rings {
                inner: Some(85.0),
                outer: Some(40.0)
            }
        );
        assert_eq!(
            label(&[agy], NOW).as_deref(),
            Some("agy 85% · 5h · Claude and GPT")
        );
        let calm = usage(
            Agent::Agy,
            vec![
                pool(10_080, 11.0, "Gemini"),
                pool(10_080, 11.0, "Claude and GPT"),
            ],
            Some(NOW),
        );
        assert_eq!(
            ring_windows(&calm, NOW)[0].model.as_deref(),
            Some("Gemini"),
            "the first pool reported wins a tie"
        );
    }

    #[test]
    fn rings_show_the_short_window_inside_and_the_week_outside() {
        let claude = usage(
            Agent::Claude,
            vec![
                window(10_080, 67.0, NOW + 5_000),
                window(300, 42.0, NOW + 100),
                UsageWindow {
                    model: Some("Fable".into()),
                    ..window(10_080, 99.0, NOW + 5_000)
                },
            ],
            Some(NOW),
        );
        assert_eq!(
            rings(&claude, NOW),
            Rings {
                inner: Some(42.0),
                outer: Some(67.0)
            },
            "per-model windows stay in the details"
        );
        let single = usage(Agent::Codex, vec![window(10_080, 12.0, NOW + 5)], Some(NOW));
        assert_eq!(
            rings(&single, NOW),
            Rings {
                inner: None,
                outer: Some(12.0)
            }
        );
        let reset = usage(
            Agent::Codex,
            vec![window(300, 80.0, NOW - 1)],
            Some(NOW - 10),
        );
        assert!(
            rings(&reset, NOW).is_empty(),
            "a reset window no longer counts"
        );
        assert!(rings(&usage(Agent::Agy, Vec::new(), None), NOW).is_empty());
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
    fn label_names_one_agent_near_its_limit_or_counts_several() {
        let agents = [
            usage(Agent::Claude, vec![window(300, 60.0, NOW + 10)], Some(NOW)),
            usage(Agent::Codex, vec![window(300, 82.4, NOW + 10)], Some(NOW)),
            usage(Agent::Agy, vec![], None),
        ];
        assert_eq!(label(&agents, NOW).as_deref(), Some("codex 82% · 5h"));
        let calm = [usage(
            Agent::Codex,
            vec![window(10_080, 79.0, NOW + 10)],
            Some(NOW),
        )];
        assert_eq!(label(&calm, NOW), None, "below 80% nothing is named");
        let both = [
            usage(
                Agent::Claude,
                vec![window(10_080, 91.0, NOW + 10)],
                Some(NOW),
            ),
            usage(Agent::Codex, vec![window(300, 85.0, NOW + 10)], Some(NOW)),
        ];
        assert_eq!(
            label(&both, NOW).as_deref(),
            Some("2 agents near their limit")
        );
    }

    #[test]
    fn one_agent_is_named_by_its_nearest_window_the_shorter_on_a_tie() {
        let week_ahead = [usage(
            Agent::Claude,
            vec![window(300, 85.0, NOW + 10), window(10_080, 92.0, NOW + 10)],
            Some(NOW),
        )];
        assert_eq!(
            label(&week_ahead, NOW).as_deref(),
            Some("claude 92% · week")
        );
        let tied = [usage(
            Agent::Claude,
            vec![window(10_080, 88.2, NOW + 10), window(300, 87.6, NOW + 10)],
            Some(NOW),
        )];
        assert_eq!(
            label(&tied, NOW).as_deref(),
            Some("claude 88% · 5h"),
            "the same number shown: the 5-hour window"
        );
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
