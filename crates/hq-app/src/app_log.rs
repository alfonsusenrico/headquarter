//! Writes the toolkit's log messages (GPUI reports failed notification deliveries and
//! denied permissions this way) to standard error. Warnings and errors by default;
//! `HQ_APP_LOG=info` or `debug` shows more. Nothing here carries prompts or output.

use std::io::Write;

use log::{Level, Log, Metadata, Record};

struct StderrLog {
    level: Level,
}

impl Log for StderrLog {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let _ = writeln!(
                std::io::stderr(),
                "headquarter-app {} {}: {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    let level = match std::env::var("HQ_APP_LOG").as_deref() {
        Ok("debug") => Level::Debug,
        Ok("info") => Level::Info,
        _ => Level::Warn,
    };
    let logger = Box::new(StderrLog { level });
    if log::set_boxed_logger(logger).is_ok() {
        log::set_max_level(level.to_level_filter());
    }
}
