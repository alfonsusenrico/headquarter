//! Why the app gave up on a host, and what the owner can do (fix-hosts-menu-reason D1, D2).

use hq_protocol::PROTOCOL_VERSION;

/// The app's own release, named when versions differ.
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TroubleKind {
    /// The two sides speak different protocols; the host names its release.
    Update {
        host_protocol: u32,
        host_version: String,
    },
    /// The host no longer knows this computer.
    PairAgain,
    /// Anything else: the host record cannot be read, or this computer's own host failed.
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostTrouble {
    pub kind: TroubleKind,
    /// The error as the app met it, for `Other`.
    pub message: String,
}

impl HostTrouble {
    pub fn update(host_protocol: u32, host_version: String) -> HostTrouble {
        HostTrouble {
            message: format!("the host runs Headquarter {host_version} (protocol {host_protocol})"),
            kind: TroubleKind::Update {
                host_protocol,
                host_version,
            },
        }
    }

    pub fn pair_again() -> HostTrouble {
        HostTrouble {
            kind: TroubleKind::PairAgain,
            message: "the host does not know this computer".to_owned(),
        }
    }

    pub fn other(message: String) -> HostTrouble {
        HostTrouble {
            kind: TroubleKind::Other,
            message,
        }
    }

    /// Whether trying again can help without the owner pairing again (D4).
    pub fn retries(&self) -> bool {
        matches!(self.kind, TroubleKind::Update { .. })
    }

    /// The Hosts menu's state word (D3).
    pub fn state_word(&self) -> &'static str {
        match self.kind {
            TroubleKind::Update { .. } => "needs an update",
            TroubleKind::PairAgain => "pair again",
            TroubleKind::Other => "not reachable",
        }
    }

    /// What the sidebar and the Hosts menu say about `host` (D2).
    pub fn line(&self, host: &str) -> String {
        match &self.kind {
            TroubleKind::Update {
                host_protocol,
                host_version,
            } if *host_protocol > PROTOCOL_VERSION => format!(
                "Update this app: {host} runs Headquarter {host_version}, this app {APP_VERSION}."
            ),
            TroubleKind::Update { host_version, .. } => format!(
                "{host} needs an update: it runs Headquarter {host_version}, this app {APP_VERSION}."
            ),
            TroubleKind::PairAgain => format!(
                "{host} no longer knows this computer. Pair it again with Connect to a Host."
            ),
            TroubleKind::Other => format!("Can't use {host}: {}", self.message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_older_host_is_named_with_both_releases() {
        let trouble = HostTrouble::update(PROTOCOL_VERSION - 1, "0.2.0-alpha.2".into());
        assert_eq!(
            trouble.line("msigs63vr"),
            format!(
                "msigs63vr needs an update: it runs Headquarter 0.2.0-alpha.2, this app {APP_VERSION}."
            )
        );
        assert_eq!(trouble.state_word(), "needs an update");
        assert!(trouble.retries());
    }

    #[test]
    fn a_newer_host_asks_to_update_this_app() {
        let trouble = HostTrouble::update(PROTOCOL_VERSION + 1, "0.9.0".into());
        assert!(
            trouble
                .line("server")
                .starts_with("Update this app: server runs Headquarter 0.9.0,"),
            "{}",
            trouble.line("server")
        );
        assert!(trouble.retries());
    }

    #[test]
    fn a_removed_computer_is_told_to_pair_again_and_not_retried() {
        let trouble = HostTrouble::pair_again();
        assert_eq!(
            trouble.line("server"),
            "server no longer knows this computer. Pair it again with Connect to a Host."
        );
        assert_eq!(trouble.state_word(), "pair again");
        assert!(!trouble.retries());
    }

    #[test]
    fn anything_else_keeps_its_reason_and_is_not_retried() {
        let trouble = HostTrouble::other("the paired host's record cannot be read".into());
        assert_eq!(
            trouble.line("server"),
            "Can't use server: the paired host's record cannot be read"
        );
        assert_eq!(trouble.state_word(), "not reachable");
        assert!(!trouble.retries());
    }

    #[test]
    fn no_line_joins_clauses_with_a_semicolon() {
        for trouble in [
            HostTrouble::update(1, "0.1.0".into()),
            HostTrouble::update(PROTOCOL_VERSION + 1, "9.0.0".into()),
            HostTrouble::pair_again(),
        ] {
            assert!(
                !trouble.line("server").contains(';'),
                "{}",
                trouble.line("server")
            );
        }
    }
}
