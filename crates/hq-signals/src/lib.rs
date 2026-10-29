//! Agent session signals for Headquarter: normalized events, the session status
//! machine, terminal escape scanning, per-provider launch adapters, quota readers,
//! and the observe-only scan of sessions started elsewhere.

pub mod adapters;
pub mod background;
pub mod claude_usage;
pub mod codex_server;
pub mod details;
pub mod event;
pub mod external;
pub mod forward;
pub mod helpers;
pub mod history;
pub mod paths;
pub mod quota;
pub mod resources;
pub mod scanner;
pub mod status;
pub mod subagents;
pub mod work;

/// Milliseconds since the Unix epoch, the time unit used by every record.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
