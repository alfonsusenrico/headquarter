//! The computers this phone paired with (add-android-app D2): a small JSON file in the
//! app's private storage. It holds no secrets: a host's key is public and a pairing
//! secret is used once and never kept.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The file's name in the folder the app gives the core.
const FILE: &str = "computers.json";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Computer {
    /// The host's endpoint id (its public key).
    pub id: String,
    /// The computer's name, as its host last reported it.
    pub name: String,
    /// The host's home relay, as it last reported it.
    pub relay: Option<String>,
    /// Direct addresses from the pairing code. Only codes without a relay carry them.
    #[serde(default)]
    pub addrs: Vec<String>,
    /// Unix milliseconds.
    pub paired_at: u64,
    pub last_seen_at: Option<u64>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Computers {
    pub list: Vec<Computer>,
    pub selected: Option<String>,
}

impl Computers {
    pub fn path(folder: &Path) -> PathBuf {
        folder.join(FILE)
    }

    /// The saved list. An absent file is an empty list.
    pub fn load(folder: &Path) -> Result<Computers, String> {
        match fs::read(Computers::path(folder)) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| format!("the list of computers is unreadable: {error}")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Computers::default()),
            Err(error) => Err(format!("cannot read the list of computers: {error}")),
        }
    }

    /// Writes the list beside the old one, then renames it over, so a crash leaves one
    /// whole file.
    pub fn save(&self, folder: &Path) -> Result<(), String> {
        let path = Computers::path(folder);
        let partial = folder.join(format!("{FILE}.partial"));
        let bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        let write = || -> std::io::Result<()> {
            let mut file = fs::File::create(&partial)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&partial, &path)
        };
        write().map_err(|error| format!("cannot save the list of computers: {error}"))
    }

    /// Adds a computer, or replaces the entry with the same host (paired again), and
    /// selects it.
    pub fn upsert(&mut self, computer: Computer) {
        self.selected = Some(computer.id.clone());
        match self.list.iter_mut().find(|known| known.id == computer.id) {
            Some(known) => *known = computer,
            None => self.list.push(computer),
        }
    }

    /// Forgets a computer. When it was selected, the first remaining one is.
    pub fn remove(&mut self, id: &str) {
        self.list.retain(|known| known.id != id);
        if self.selected.as_deref() == Some(id) {
            self.selected = self.list.first().map(|first| first.id.clone());
        }
    }

    pub fn select(&mut self, id: &str) -> bool {
        let known = self.list.iter().any(|known| known.id == id);
        if known {
            self.selected = Some(id.to_owned());
        }
        known
    }

    pub fn selected(&self) -> Option<&Computer> {
        let id = self.selected.as_deref()?;
        self.list.iter().find(|known| known.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Computer> {
        self.list.iter_mut().find(|known| known.id == id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn computer(id: &str, name: &str) -> Computer {
        Computer {
            id: id.to_owned(),
            name: name.to_owned(),
            relay: Some("https://relay.example.com./".to_owned()),
            addrs: Vec::new(),
            paired_at: 1,
            last_seen_at: None,
        }
    }

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("hq-mobile-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn the_list_survives_a_round_trip_and_starts_empty() {
        let folder = folder("round-trip");
        assert_eq!(Computers::load(&folder).unwrap(), Computers::default());
        let mut computers = Computers::default();
        computers.upsert(computer("a", "MacBook"));
        computers.upsert(computer("b", "Studio"));
        computers.save(&folder).unwrap();
        let read = Computers::load(&folder).unwrap();
        assert_eq!(read, computers);
        assert_eq!(read.selected().unwrap().name, "Studio");
        assert!(!folder.join("computers.json.partial").exists());
        fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn pairing_again_replaces_the_entry_and_forgetting_moves_the_selection() {
        let mut computers = Computers::default();
        computers.upsert(computer("a", "MacBook"));
        computers.upsert(computer("b", "Studio"));
        computers.upsert(computer("a", "MacBook Pro"));
        assert_eq!(computers.list.len(), 2);
        assert_eq!(computers.list[0].name, "MacBook Pro");
        assert_eq!(computers.selected.as_deref(), Some("a"));
        computers.remove("a");
        assert_eq!(computers.selected.as_deref(), Some("b"));
        computers.remove("b");
        assert_eq!(computers.selected, None);
        assert!(!computers.select("gone"));
    }

    #[test]
    fn an_unreadable_file_is_reported_not_replaced() {
        let folder = folder("broken");
        fs::write(Computers::path(&folder), b"{not json").unwrap();
        assert!(Computers::load(&folder).unwrap_err().contains("unreadable"));
        fs::remove_dir_all(folder).unwrap();
    }
}
