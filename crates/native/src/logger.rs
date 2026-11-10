use std::io::Write;

use log::{LevelFilter, Log, Metadata, Record};

/// Writes `[target][LEVEL] message` lines to stderr, so the backend's
/// `scan extract` lines and warnings are visible.
struct Stderr;

impl Log for Stderr {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            eprintln!(
                "[{}][{}] {}",
                record.target(),
                record.level(),
                record.args()
            );
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

pub fn install() {
    if log::set_logger(&Stderr).is_ok() {
        log::set_max_level(LevelFilter::Info);
    }
}
