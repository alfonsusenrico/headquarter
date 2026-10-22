//! Coding-agent CLIs Headquarter knows about. Adding an agent is adding a row.

use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct KnownAgent {
    pub id: &'static str,
    pub name: &'static str,
    pub executables: &'static [&'static str],
    /// Install folders specific to this agent, relative to the home folder.
    #[serde(skip)]
    pub home_dirs: &'static [&'static str],
    /// Whether Headquarter can host this agent's sessions yet.
    pub hostable: bool,
    /// The vendor's own published mark in its colours, fetched once onto the owner's
    /// machine and drawn unaltered; never bundled (redesign-session-modal D5).
    #[serde(skip)]
    pub mark_color: Option<MarkSource>,
    /// The vendor's own one-colour version of that mark, drawn in black or white.
    #[serde(skip)]
    pub mark_mono: Option<MarkSource>,
}

/// Where a vendor publishes a mark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarkSource {
    pub url: &'static str,
    /// The file to take from the zip archive at `url`; `None` when `url` is the file.
    pub entry: Option<&'static str>,
}

const fn file(url: &'static str) -> Option<MarkSource> {
    Some(MarkSource { url, entry: None })
}

pub const KNOWN_AGENTS: &[KnownAgent] = &[
    KnownAgent {
        id: "claude",
        name: "Claude Code",
        executables: &["claude"],
        home_dirs: &[".claude/local", ".local/bin"],
        hostable: true,
        mark_color: file("https://assets.claude.com/95a868946ac8a31e5ff832e2899f294aa368b836.png"),
        // Anthropic publishes the spark in its colour only.
        mark_mono: None,
    },
    KnownAgent {
        id: "codex",
        name: "Codex",
        executables: &["codex"],
        home_dirs: &[],
        hostable: true,
        mark_color: file("https://developers.openai.com/favicon.png"),
        mark_mono: Some(MarkSource {
            url: "https://cdn.openai.com/brand/OpenAI-Logos-2025.zip",
            entry: Some("OpenAI-logos(new)/SVGs/OpenAI-black-monoblossom.svg"),
        }),
    },
    KnownAgent {
        id: "agy",
        name: "Antigravity CLI",
        executables: &["agy"],
        home_dirs: &[".local/bin"],
        hostable: true,
        mark_color: file(
            "https://antigravity.google/assets/image/brand/antigravity-icon__full-color.svg",
        ),
        mark_mono: file(
            "https://antigravity.google/assets/image/brand/antigravity-icon__one-color.svg",
        ),
    },
    KnownAgent {
        id: "gemini",
        name: "Gemini CLI",
        executables: &["gemini"],
        home_dirs: &[],
        hostable: false,
        mark_color: None,
        mark_mono: None,
    },
    KnownAgent {
        id: "opencode",
        name: "opencode",
        executables: &["opencode"],
        home_dirs: &[".opencode/bin"],
        hostable: false,
        mark_color: None,
        mark_mono: None,
    },
    KnownAgent {
        id: "muse",
        name: "Muse Code",
        executables: &["muse"],
        home_dirs: &[".local/bin"],
        hostable: false,
        mark_color: None,
        mark_mono: None,
    },
    KnownAgent {
        id: "amp",
        name: "Amp",
        executables: &["amp"],
        home_dirs: &[".amp/bin"],
        hostable: false,
        mark_color: None,
        mark_mono: None,
    },
    KnownAgent {
        id: "cursor-agent",
        name: "Cursor Agent",
        executables: &["cursor-agent"],
        home_dirs: &[".local/bin"],
        hostable: false,
        mark_color: None,
        mark_mono: None,
    },
    KnownAgent {
        id: "copilot",
        name: "GitHub Copilot CLI",
        executables: &["copilot"],
        home_dirs: &[],
        hostable: false,
        mark_color: None,
        mark_mono: None,
    },
    KnownAgent {
        id: "goose",
        name: "Goose",
        executables: &["goose"],
        home_dirs: &[".local/bin"],
        hostable: false,
        mark_color: None,
        mark_mono: None,
    },
    KnownAgent {
        id: "crush",
        name: "Crush",
        executables: &["crush"],
        home_dirs: &[],
        hostable: false,
        mark_color: None,
        mark_mono: None,
    },
    KnownAgent {
        id: "qwen",
        name: "Qwen Code",
        executables: &["qwen"],
        home_dirs: &[],
        hostable: false,
        mark_color: None,
        mark_mono: None,
    },
];

pub fn known(id: &str) -> Option<&'static KnownAgent> {
    KNOWN_AGENTS.iter().find(|agent| agent.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hostable_agents_have_an_official_mark_over_https() {
        for agent in KNOWN_AGENTS.iter().filter(|agent| agent.hostable) {
            let color = agent.mark_color.expect(agent.id);
            for source in [Some(color), agent.mark_mono].into_iter().flatten() {
                assert!(source.url.starts_with("https://"), "{}", source.url);
            }
        }
    }

    #[test]
    fn ids_are_unique_and_hostable_agents_match_the_host() {
        let mut ids: Vec<&str> = KNOWN_AGENTS.iter().map(|agent| agent.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), KNOWN_AGENTS.len());
        let hostable: Vec<&str> = KNOWN_AGENTS
            .iter()
            .filter(|agent| agent.hostable)
            .map(|agent| agent.id)
            .collect();
        assert_eq!(hostable, ["claude", "codex", "agy"]);
        assert!(known("opencode").is_some_and(|agent| !agent.hostable));
        assert!(known("nope").is_none());
    }
}
