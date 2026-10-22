//! What machine Headquarter runs on, from cheap local facts only.

use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Os {
    Macos,
    Linux,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Family {
    Debian,
    Fedora,
    Arch,
    Suse,
    Alpine,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Distro {
    pub id: String,
    pub family: Family,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Platform {
    pub os: Os,
    pub arch: String,
    pub distro: Option<Distro>,
    pub wsl: bool,
    pub homebrew: Option<PathBuf>,
}

pub fn detect() -> Platform {
    let os = match std::env::consts::OS {
        "macos" => Os::Macos,
        "linux" => Os::Linux,
        _ => Os::Other,
    };
    let read = |path: &str| std::fs::read_to_string(path).ok();
    from_facts(Facts {
        os,
        arch: std::env::consts::ARCH,
        os_release: read("/etc/os-release").as_deref(),
        proc_version: read("/proc/version").as_deref(),
        wsl_env: std::env::var_os("WSL_DISTRO_NAME").is_some(),
        homebrew_env: std::env::var_os("HOMEBREW_PREFIX").map(PathBuf::from),
        exists: &|path: &Path| path.exists(),
    })
}

pub struct Facts<'a> {
    pub os: Os,
    pub arch: &'a str,
    pub os_release: Option<&'a str>,
    pub proc_version: Option<&'a str>,
    pub wsl_env: bool,
    pub homebrew_env: Option<PathBuf>,
    pub exists: &'a dyn Fn(&Path) -> bool,
}

pub fn from_facts(facts: Facts<'_>) -> Platform {
    let linux = facts.os == Os::Linux;
    let distro = if linux {
        facts.os_release.and_then(parse_os_release)
    } else {
        None
    };
    let wsl = linux
        && (facts.wsl_env
            || facts
                .proc_version
                .is_some_and(|version| version.to_lowercase().contains("microsoft")));
    let candidates: Vec<PathBuf> = match (facts.os, facts.arch) {
        (Os::Macos, "aarch64") => vec!["/opt/homebrew".into(), "/usr/local".into()],
        (Os::Macos, _) => vec!["/usr/local".into(), "/opt/homebrew".into()],
        (Os::Linux, _) => vec!["/home/linuxbrew/.linuxbrew".into()],
        (Os::Other, _) => vec![],
    };
    let homebrew = facts
        .homebrew_env
        .into_iter()
        .chain(candidates)
        .find(|prefix| (facts.exists)(&prefix.join("bin").join("brew")));
    Platform {
        os: facts.os,
        arch: facts.arch.to_owned(),
        distro,
        wsl,
        homebrew,
    }
}

/// Reads `ID`, `ID_LIKE`, and `PRETTY_NAME` from an os-release file.
fn parse_os_release(text: &str) -> Option<Distro> {
    let value = |key: &str| {
        text.lines().find_map(|line| {
            let (name, value) = line.split_once('=')?;
            (name.trim() == key).then(|| value.trim().trim_matches(['"', '\'']).to_owned())
        })
    };
    let id = value("ID")?.to_lowercase();
    let like = value("ID_LIKE").unwrap_or_default().to_lowercase();
    let family_of = |name: &str| match name {
        "debian" | "ubuntu" => Some(Family::Debian),
        "fedora" | "rhel" | "centos" => Some(Family::Fedora),
        "arch" => Some(Family::Arch),
        "suse" | "opensuse" | "sles" => Some(Family::Suse),
        "alpine" => Some(Family::Alpine),
        _ => None,
    };
    let family = std::iter::once(id.as_str())
        .chain(like.split_whitespace())
        .find_map(family_of)
        .unwrap_or(Family::Other);
    Some(Distro {
        id,
        family,
        name: value("PRETTY_NAME"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linux(os_release: &str, proc_version: &str) -> Platform {
        from_facts(Facts {
            os: Os::Linux,
            arch: "x86_64",
            os_release: Some(os_release),
            proc_version: Some(proc_version),
            wsl_env: false,
            homebrew_env: None,
            exists: &|_| false,
        })
    }

    #[test]
    fn distribution_families() {
        let cases = [
            ("ID=ubuntu\nID_LIKE=debian\n", Family::Debian),
            ("ID=\"fedora\"\n", Family::Fedora),
            ("ID=rocky\nID_LIKE=\"rhel centos fedora\"\n", Family::Fedora),
            ("ID=arch\n", Family::Arch),
            ("ID=endeavouros\nID_LIKE=arch\n", Family::Arch),
            (
                "ID=opensuse-tumbleweed\nID_LIKE=\"opensuse suse\"\n",
                Family::Suse,
            ),
            ("ID=alpine\n", Family::Alpine),
            ("ID=nixos\n", Family::Other),
        ];
        for (text, family) in cases {
            assert_eq!(
                linux(text, "Linux version 6.8").distro.unwrap().family,
                family,
                "{text}"
            );
        }
    }

    #[test]
    fn wsl_is_detected_from_the_kernel_or_environment() {
        let platform = linux(
            "ID=ubuntu\nPRETTY_NAME=\"Ubuntu 24.04 LTS\"\n",
            "Linux version 5.15.153.1-microsoft-standard-WSL2",
        );
        assert!(platform.wsl);
        assert_eq!(
            platform.distro.unwrap().name.as_deref(),
            Some("Ubuntu 24.04 LTS")
        );
        assert!(!linux("ID=ubuntu\n", "Linux version 6.8.0-generic").wsl);
        let from_env = from_facts(Facts {
            os: Os::Linux,
            arch: "x86_64",
            os_release: None,
            proc_version: None,
            wsl_env: true,
            homebrew_env: None,
            exists: &|_| false,
        });
        assert!(from_env.wsl);
        assert_eq!(from_env.distro, None);
    }

    #[test]
    fn homebrew_prefix_follows_the_machine() {
        let has = |prefixes: &'static [&'static str]| {
            move |path: &Path| prefixes.iter().any(|prefix| path.starts_with(prefix))
        };
        let apple = has(&["/opt/homebrew", "/usr/local"]);
        let mac = |arch| {
            from_facts(Facts {
                os: Os::Macos,
                arch,
                os_release: None,
                proc_version: None,
                wsl_env: false,
                homebrew_env: None,
                exists: &apple,
            })
        };
        assert_eq!(mac("aarch64").homebrew, Some("/opt/homebrew".into()));
        assert_eq!(mac("x86_64").homebrew, Some("/usr/local".into()));
        let none = from_facts(Facts {
            os: Os::Macos,
            arch: "aarch64",
            os_release: None,
            proc_version: None,
            wsl_env: false,
            homebrew_env: None,
            exists: &|_| false,
        });
        assert_eq!(none.homebrew, None);
        let linuxbrew = has(&["/home/linuxbrew"]);
        let on_linux = from_facts(Facts {
            os: Os::Linux,
            arch: "x86_64",
            os_release: Some("ID=debian\n"),
            proc_version: None,
            wsl_env: false,
            homebrew_env: None,
            exists: &linuxbrew,
        });
        assert_eq!(on_linux.homebrew, Some("/home/linuxbrew/.linuxbrew".into()));
    }
}
