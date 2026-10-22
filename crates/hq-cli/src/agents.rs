//! `hq agents`: the platform and every known agent CLI found on this machine.

use std::path::Path;

use hq_config::{Settings, SettingsError};
use hq_discovery::platform::{Family, Os, Platform};
use hq_discovery::search::{Found, Source, VERSION_TIMEOUT};
use hq_discovery::{AgentReport, KNOWN_AGENTS, SearchContext, add_versions, search};
use serde_json::json;

pub struct Discovered {
    pub platform: Platform,
    pub login_shell: LoginShell,
    pub settings_error: Option<SettingsError>,
    pub settings: Settings,
    pub reports: Vec<AgentReport>,
}

pub enum LoginShell {
    Read,
    Unavailable,
    Skipped,
}

pub fn discover() -> Discovered {
    let path = hq_config::settings_path();
    let ids: Vec<&str> = KNOWN_AGENTS.iter().map(|agent| agent.id).collect();
    let (settings, settings_error) = match path.as_deref().map(|path| hq_config::load(path, &ids)) {
        Some(Ok(settings)) => (settings, None),
        Some(Err(error)) => (Settings::default(), Some(error)),
        None => (Settings::default(), None),
    };
    let (login_path, login_shell) = if settings.login_shell {
        match hq_discovery::shell::login_shell_path(hq_discovery::shell::LOGIN_SHELL_TIMEOUT) {
            Some(path) => (Some(path), LoginShell::Read),
            None => (None, LoginShell::Unavailable),
        }
    } else {
        (None, LoginShell::Skipped)
    };
    let platform = hq_discovery::platform::detect();
    let context = SearchContext::current(platform.clone(), login_path);
    let mut reports: Vec<AgentReport> = KNOWN_AGENTS
        .iter()
        .map(|agent| search(&context, agent, settings.agent_path(agent.id)))
        .collect();
    add_versions(&mut reports, VERSION_TIMEOUT);
    Discovered {
        platform,
        login_shell,
        settings_error,
        settings,
        reports,
    }
}

pub fn render_json(discovered: &Discovered) -> String {
    let agents: Vec<serde_json::Value> = discovered
        .reports
        .iter()
        .map(|report| {
            json!({
                "id": report.agent.id,
                "name": report.agent.name,
                "hostable": report.agent.hostable,
                "enabled": discovered.settings.agent_enabled(report.agent.id),
                "installed": report.used.is_some(),
                "used": report.used_path(),
                "version": report.version,
                "found": report.found,
                "pin_problem": report.pin_problem,
                "searched": report.searched,
            })
        })
        .collect();
    let value = json!({
        "platform": discovered.platform,
        "login_shell_path": match discovered.login_shell {
            LoginShell::Read => "read",
            LoginShell::Unavailable => "unavailable",
            LoginShell::Skipped => "skipped",
        },
        "settings_error": discovered.settings_error.as_ref().map(ToString::to_string),
        "settings_warnings": discovered
            .settings
            .warnings
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        "agents": agents,
    });
    serde_json::to_string_pretty(&value).unwrap_or_default()
}

pub fn render_text(discovered: &Discovered) -> String {
    let home = hq_signals::paths::home().map(|home| home.to_string_lossy().into_owned());
    let short = |path: &Path| shorten(&path.to_string_lossy(), home.as_deref());
    let mut out = format!("Platform: {}\n", platform_text(&discovered.platform));
    out.push_str(match discovered.login_shell {
        LoginShell::Read => "Login shell PATH: read\n",
        LoginShell::Unavailable => "Login shell PATH: unavailable (using known folders)\n",
        LoginShell::Skipped => "Login shell PATH: skipped by settings\n",
    });
    if let Some(error) = &discovered.settings_error {
        out.push_str(&format!("Settings error: {error}\n"));
    }
    for warning in &discovered.settings.warnings {
        out.push_str(&format!("Settings warning: {warning}\n"));
    }
    out.push('\n');
    for report in &discovered.reports {
        let agent = report.agent;
        let label = format!("{} ({})", agent.name, agent.id);
        let support = if !discovered.settings.agent_enabled(agent.id) {
            "turned off in settings"
        } else if agent.hostable {
            "hostable"
        } else {
            "found, not yet supported"
        };
        if let Some(problem) = &report.pin_problem {
            out.push_str(&format!("{label:<34} {problem}\n"));
            continue;
        }
        let Some(used) = report.used else {
            out.push_str(&format!("{label:<34} not installed\n"));
            continue;
        };
        let version = report.version.as_deref().unwrap_or("version unknown");
        out.push_str(&format!("{label:<34} {support} · {version}\n"));
        for (index, found) in report.found.iter().enumerate() {
            let role = if index == used { "used" } else { "also" };
            out.push_str(&format!(
                "  {role}  {}  [{}]\n",
                location(found, &short),
                source_text(found.source)
            ));
        }
    }
    out
}

fn location(found: &Found, short: &dyn Fn(&Path) -> String) -> String {
    match &found.target {
        Some(target) => format!("{} -> {}", short(&found.path), short(target)),
        None => short(&found.path),
    }
}

fn source_text(source: Source) -> &'static str {
    match source {
        Source::Settings => "settings file",
        Source::ProcessPath => "PATH",
        Source::LoginShell => "login shell PATH",
        Source::KnownFolder => "known folder",
    }
}

fn platform_text(platform: &Platform) -> String {
    let mut parts = vec![match platform.os {
        Os::Macos => format!("macOS {}", platform.arch),
        Os::Linux => format!("Linux {}", platform.arch),
        Os::Other => format!("unsupported OS ({})", platform.arch),
    }];
    if let Some(distro) = &platform.distro {
        let family = match distro.family {
            Family::Debian => "debian",
            Family::Fedora => "fedora",
            Family::Arch => "arch",
            Family::Suse => "suse",
            Family::Alpine => "alpine",
            Family::Other => "other",
        };
        let name = distro.name.as_deref().unwrap_or(&distro.id);
        parts.push(format!("{name} ({family} family)"));
    }
    if platform.wsl {
        parts.push("WSL".to_owned());
    }
    if let Some(prefix) = &platform.homebrew {
        parts.push(format!("Homebrew {}", prefix.display()));
    }
    parts.join(" · ")
}

fn shorten(path: &str, home: Option<&str>) -> String {
    match home {
        Some(home) if path.starts_with(&format!("{home}/")) => format!("~{}", &path[home.len()..]),
        _ => path.to_owned(),
    }
}
