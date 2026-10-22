//! The host's log: ids, states, counts, and durations only. Callers must never pass
//! terminal bytes, hook payloads, or prompt text.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

/// When the log grows past this, it is moved to `<name>.1` and started afresh.
const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

pub struct Log {
    path: PathBuf,
    file: Mutex<Option<File>>,
}

impl Log {
    pub fn open(path: PathBuf) -> Log {
        let file = Self::open_file(&path);
        Log {
            path,
            file: Mutex::new(file),
        }
    }

    fn open_file(path: &PathBuf) -> Option<File> {
        OpenOptions::new().create(true).append(true).open(path).ok()
    }

    /// Logging is best effort: a full disk must not stop the host.
    pub fn line(&self, message: &str) {
        let Ok(mut guard) = self.file.lock() else {
            return;
        };
        let too_big = guard
            .as_ref()
            .and_then(|file| file.metadata().ok())
            .is_some_and(|metadata| metadata.len() > MAX_LOG_BYTES);
        if too_big {
            let rotated = self.path.with_extension("log.1");
            let _ = std::fs::rename(&self.path, rotated);
            *guard = Self::open_file(&self.path);
        }
        if let Some(file) = guard.as_mut() {
            let _ = writeln!(file, "{} {message}", hq_signals::now_ms());
        }
    }
}
