use std::io::Write;

use log::{LevelFilter, Log, Metadata, Record};

/// Writes `[target][LEVEL] message` lines to stderr, so the backend's
/// `scan extract` lines and warnings are visible. `RUST_LOG=debug` (or
/// `trace`) adds this crate's own lines of that level, not GPUI's.
struct Stderr;

impl Log for Stderr {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
            && (metadata.level() <= LevelFilter::Info
                || metadata.target().starts_with(env!("CARGO_CRATE_NAME")))
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
        let level = std::env::var("RUST_LOG")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(LevelFilter::Info)
            .max(LevelFilter::Info);
        log::set_max_level(level);
    }
}
