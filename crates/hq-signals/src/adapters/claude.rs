//! Claude Code: hooks and a status line passed with `--settings`, and a session id
//! chosen up front with `--session-id` (or `--resume` to continue one). `--settings` adds to the owner's settings,
//! so their own hooks keep running; their status line is chained by the wrapper.

use std::path::Path;

use serde_json::{Value, json};

use super::{Conversation, LaunchContext, LaunchSpec, OwnerSettings, shell_quote};
use crate::background::Background;
use crate::paths;

const HOOK_EVENTS: [&str; 9] = [
    "SessionStart",
    "UserPromptSubmit",
    "PreToolUse",
    "PostToolUse",
    "PermissionRequest",
    "Notification",
    "Stop",
    "StopFailure",
    "SessionEnd",
];

pub fn launch_spec(ctx: &LaunchContext<'_>, owner: &OwnerSettings) -> LaunchSpec {
    let exe = shell_quote(&ctx.helper_exe.to_string_lossy());
    let forward = format!(
        "{exe} hook-forward --socket {} --agent claude",
        shell_quote(&ctx.hook_socket.to_string_lossy())
    );
    let mut status_line = format!(
        "{exe} hook-statusline --socket {}",
        shell_quote(&ctx.hook_socket.to_string_lossy())
    );
    if let Some(command) = &owner.claude_status_line {
        status_line.push_str(" --chain ");
        status_line.push_str(&shell_quote(command));
    }

    let hook_groups = json!([{ "hooks": [{ "type": "command", "command": forward }] }]);
    let hooks: serde_json::Map<String, Value> = HOOK_EVENTS
        .iter()
        .map(|event| ((*event).to_owned(), hook_groups.clone()))
        .collect();
    let mut settings = json!({
        "hooks": hooks,
        "statusLine": { "type": "command", "command": status_line },
    });
    // Its default theme is dark, and a session the host starts has no window to answer
    // its `auto` theme's question: told light or dark here. A plain light, dark or auto
    // the owner set follows the terminal too; a colour-blind, ANSI or custom theme is
    // theirs (fix-light-theme-contrast D2, owner 2026-10-29).
    if follows_the_terminal(owner.claude_theme.as_deref())
        && let Some(background) = ctx.background
    {
        settings["theme"] = json!(match background {
            Background::Light => "light",
            Background::Dark => "dark",
        });
    }

    let mut args = match ctx.conversation {
        Conversation::New { id } => vec!["--session-id".to_owned(), id.to_owned()],
        Conversation::Resume(Some(id)) => vec!["--resume".to_owned(), id.to_owned()],
        Conversation::Resume(None) => vec!["--resume".to_owned()],
    };
    args.push("--settings".to_owned());
    args.push(settings.to_string());

    LaunchSpec {
        program: "claude".to_owned(),
        args,
        structured: true,
    }
}

/// The owner's status line command, by Claude Code's precedence: local project
/// settings, shared project settings, then user settings. Read-only.
pub fn owner_status_line(cwd: &Path) -> Option<String> {
    let mut candidates = vec![
        cwd.join(".claude").join("settings.local.json"),
        cwd.join(".claude").join("settings.json"),
    ];
    if let Some(config) = paths::claude_config_dir() {
        candidates.push(config.join("settings.json"));
    }
    candidates
        .iter()
        .find_map(|path| status_line_from(&std::fs::read_to_string(path).ok()?))
}

/// The theme the owner set in Claude Code: in its settings files, by the same precedence
/// as the status line, then in its global configuration. Read-only.
pub fn owner_theme(cwd: &Path) -> Option<String> {
    let mut candidates = vec![
        cwd.join(".claude").join("settings.local.json"),
        cwd.join(".claude").join("settings.json"),
    ];
    if let Some(config) = paths::claude_config_dir() {
        candidates.push(config.join("settings.json"));
    }
    candidates.extend(paths::claude_global_config());
    candidates
        .iter()
        .find_map(|path| theme_from(&std::fs::read_to_string(path).ok()?))
}

fn follows_the_terminal(owner_theme: Option<&str>) -> bool {
    owner_theme.is_none_or(|theme| matches!(theme, "light" | "dark" | "auto"))
}

fn theme_from(settings: &str) -> Option<String> {
    let value: Value = serde_json::from_str(settings).ok()?;
    value
        .get("theme")?
        .as_str()
        .filter(|theme| !theme.trim().is_empty())
        .map(str::to_owned)
}

fn status_line_from(settings: &str) -> Option<String> {
    let value: Value = serde_json::from_str(settings).ok()?;
    value
        .get("statusLine")?
        .get("command")?
        .as_str()
        .filter(|command| !command.trim().is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn spec_for(owner: &OwnerSettings, conversation: Conversation<'_>) -> LaunchSpec {
        let exe = PathBuf::from("/opt/hq probe/hq-probe");
        let socket = PathBuf::from("/tmp/hq.sock");
        launch_spec(
            &LaunchContext {
                helper_exe: &exe,
                hook_socket: &socket,
                conversation,
                background: None,
            },
            owner,
        )
    }

    fn spec(owner: &OwnerSettings) -> LaunchSpec {
        spec_for(owner, Conversation::New { id: "0b1c" })
    }

    fn settings(spec: &LaunchSpec) -> Value {
        assert_eq!(spec.args[0], "--session-id");
        assert_eq!(spec.args[1], "0b1c");
        assert_eq!(spec.args[2], "--settings");
        serde_json::from_str(&spec.args[3]).unwrap()
    }

    #[test]
    fn every_hook_forwards_to_the_socket() {
        let spec = spec(&OwnerSettings::default());
        let settings = settings(&spec);
        for event in HOOK_EVENTS {
            let command = settings["hooks"][event][0]["hooks"][0]["command"]
                .as_str()
                .unwrap();
            assert_eq!(
                command,
                "'/opt/hq probe/hq-probe' hook-forward --socket '/tmp/hq.sock' --agent claude"
            );
        }
        assert!(spec.structured);
    }

    #[test]
    fn status_line_chains_the_owner_command() {
        let owner = OwnerSettings {
            claude_status_line: Some("bash ~/.claude/status.sh 'x'".into()),
            claude_theme: None,
        };
        let settings = settings(&spec(&owner));
        assert_eq!(
            settings["statusLine"]["command"],
            r#"'/opt/hq probe/hq-probe' hook-statusline --socket '/tmp/hq.sock' --chain 'bash ~/.claude/status.sh '\''x'\'''"#
        );
    }

    #[test]
    fn status_line_without_owner_command() {
        let settings = settings(&spec(&OwnerSettings::default()));
        assert_eq!(
            settings["statusLine"]["command"],
            "'/opt/hq probe/hq-probe' hook-statusline --socket '/tmp/hq.sock'"
        );
    }

    #[test]
    fn the_theme_follows_the_terminal_unless_the_owner_chose_one() {
        let exe = PathBuf::from("/opt/hq probe/hq-probe");
        let socket = PathBuf::from("/tmp/hq.sock");
        let with = |background, owner: &OwnerSettings| {
            settings(&launch_spec(
                &LaunchContext {
                    helper_exe: &exe,
                    hook_socket: &socket,
                    conversation: Conversation::New { id: "0b1c" },
                    background,
                },
                owner,
            ))
        };
        let plain = OwnerSettings::default();
        assert_eq!(with(Some(Background::Light), &plain)["theme"], "light");
        assert_eq!(with(Some(Background::Dark), &plain)["theme"], "dark");
        assert!(with(None, &plain).get("theme").is_none(), "not known yet");
        let plain_dark = OwnerSettings {
            claude_theme: Some("dark".into()),
            ..OwnerSettings::default()
        };
        assert_eq!(with(Some(Background::Light), &plain_dark)["theme"], "light");
        let chose = OwnerSettings {
            claude_theme: Some("dark-daltonized".into()),
            ..OwnerSettings::default()
        };
        assert!(with(Some(Background::Light), &chose).get("theme").is_none());
        assert_eq!(
            theme_from(r#"{"theme":"light-ansi"}"#).as_deref(),
            Some("light-ansi")
        );
        assert_eq!(theme_from(r#"{"numStartups":3}"#), None);
    }

    #[test]
    fn resume_uses_the_agent_resume_flag() {
        let known = spec_for(&OwnerSettings::default(), Conversation::Resume(Some("abc")));
        assert_eq!(known.args[..2], ["--resume", "abc"]);
        assert_eq!(known.args[2], "--settings");
        let picker = spec_for(&OwnerSettings::default(), Conversation::Resume(None));
        assert_eq!(picker.args[..2], ["--resume", "--settings"]);
    }

    #[test]
    fn owner_status_line_parsing() {
        assert_eq!(
            status_line_from(r#"{"statusLine":{"type":"command","command":"x.sh"}}"#),
            Some("x.sh".into())
        );
        assert_eq!(status_line_from(r#"{"statusLine":{"command":"  "}}"#), None);
        assert_eq!(status_line_from("{}"), None);
        assert_eq!(status_line_from("not json"), None);
    }
}
