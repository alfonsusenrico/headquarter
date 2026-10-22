//! Human-readable output for `quota` and `scan`.

use std::process::ExitCode;
use std::time::Duration;

use hq_signals::event::Agent;
use hq_signals::external;
use hq_signals::quota::{self, QuotaReport};

const CODEX_TIMEOUT: Duration = Duration::from_secs(15);

pub fn quota() {
    let now = hq_signals::now_ms() / 1000;
    let claude = match hq_signals::helpers::claude_snapshot_path() {
        Some(path) => quota::read_snapshot(&path, Agent::Claude),
        None => QuotaReport::Unavailable {
            provider: Agent::Claude,
            reason: "home directory unknown".to_owned(),
        },
    };
    let reports = [
        claude,
        quota::read_codex(std::path::Path::new("codex"), CODEX_TIMEOUT, now).0,
        quota::read_agy(),
    ];
    for report in reports {
        match report {
            QuotaReport::Available(reading) => {
                println!(
                    "{:<7} read {} ago",
                    reading.provider,
                    duration(now.saturating_sub(reading.read_at))
                );
                for window in &reading.windows {
                    let reset = match window.resets_at {
                        Some(_) if window.is_stale(reading.read_at, now) => {
                            "stale: window has reset since this reading".to_owned()
                        }
                        Some(at) => format!("resets in {}", duration(at.saturating_sub(now))),
                        None => "reset time unknown".to_owned(),
                    };
                    println!(
                        "        {:<4} {:>5.1}% used  {reset}",
                        quota::window_label(window.duration_mins),
                        window.used_percent
                    );
                }
            }
            QuotaReport::Unavailable { provider, reason } => {
                println!("{provider:<7} unavailable: {reason}");
            }
        }
    }
}

pub fn scan(json: bool) -> ExitCode {
    let report = external::scan(
        &mut external::ScanMemory::default(),
        &[std::process::id()],
        Some(std::path::Path::new("claude")),
        true,
    );
    if json {
        return match serde_json::to_string_pretty(&report) {
            Ok(text) => {
                println!("{text}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("hq-probe: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let now = hq_signals::now_ms() / 1000;
    let groups = external::by_folder(&report.sessions);
    let mut folders: Vec<&String> = groups.keys().collect();
    folders.sort();
    for folder in folders {
        println!("{folder}");
        for session in &groups[folder] {
            let status = session
                .status
                .map(|status| status.label())
                .unwrap_or("unknown");
            let age = session
                .last_activity_at
                .or(session.started_at)
                .map(|at| format!("{} ago", duration(now.saturating_sub(at))))
                .unwrap_or_default();
            let pid = session
                .pid
                .map(|pid| format!("pid {pid}"))
                .unwrap_or_default();
            println!(
                "  {:<7} {:<19} {:<10} {:<10} external",
                session.agent, status, age, pid
            );
        }
    }
    for failure in &report.failures {
        println!("source failed: {}: {}", failure.source, failure.error);
    }
    ExitCode::SUCCESS
}

/// Compact relative duration in the sidebar's style: `45s`, `12m`, `3h 5m`, `2d 4h`.
fn duration(secs: u64) -> String {
    match secs {
        0..60 => format!("{secs}s"),
        60..3_600 => format!("{}m", secs / 60),
        3_600..86_400 => format!("{}h {}m", secs / 3_600, secs % 3_600 / 60),
        _ => format!("{}d {}h", secs / 86_400, secs % 86_400 / 3_600),
    }
}

#[cfg(test)]
mod tests {
    use super::duration;

    #[test]
    fn durations() {
        assert_eq!(duration(5), "5s");
        assert_eq!(duration(720), "12m");
        assert_eq!(duration(3 * 3_600 + 300), "3h 5m");
        assert_eq!(duration(2 * 86_400 + 4 * 3_600), "2d 4h");
    }
}
