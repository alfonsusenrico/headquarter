//! Finding coding-agent CLIs on any macOS or Linux machine: platform detection, the
//! owner's login-shell `PATH`, the platform's usual install folders, and a table of
//! known agents. See `openspec/specs/agent-discovery/`.

pub mod agents;
pub mod platform;
pub mod search;
pub mod shell;

pub use agents::{KNOWN_AGENTS, KnownAgent, MarkSource, known};
pub use platform::Platform;
pub use search::{AgentReport, SearchContext, add_versions, search};
