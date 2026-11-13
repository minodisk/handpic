use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use log::{LevelFilter, Log, Metadata, Record};

use crate::datadir::IDENTIFIER;

pub const LOG_FILE: &str = "Handpic.log";

/// The Tauri app's `max_file_size`: the next write that would pass it starts
/// the file anew, as its `RotationStrategy::KeepOne` does.
pub const MAX_FILE_SIZE: u64 = 1_000_000;

/// Writes `[target][LEVEL] message` lines to stderr, so the backend's
/// `scan extract` lines and warnings are visible, and the same records as
/// `[YYYY-MM-DD][HH:MM:SS][target][LEVEL] message` (UTC) to `Handpic.log`.
/// `RUST_LOG=debug` (or `trace`) adds this crate's own lines of that level,
/// not GPUI's.
struct Logger {
    file: Option<Mutex<Sink>>,
}

impl Log for Logger {
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
            if let Some(file) = &self.file {
                let secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                let line = line(secs, record.target(), record.level(), record.args());
                if let Ok(mut file) = file.lock() {
                    file.write(&line);
                }
            }
        }
    }

    fn flush(&self) {
        let _ = io::stderr().flush();
    }
}

static LOGGER: OnceLock<Logger> = OnceLock::new();

/// Install the logger, writing the file at `flag` when given, else at
/// `log_path`'s default; a file that cannot be opened is reported once on
/// stderr and the app logs to stderr only.
pub fn install(flag: Option<PathBuf>) {
    let base = if cfg!(target_os = "macos") {
        dirs::home_dir().map(|home| home.join("Library").join("Logs"))
    } else {
        dirs::data_local_dir()
    };
    let file = match log_path(flag, base.as_deref()) {
        Some(path) => match Sink::open(&path, MAX_FILE_SIZE) {
            Ok(sink) => Some(Mutex::new(sink)),
            Err(e) => {
                eprintln!("cannot open the log file {}: {e}", path.display());
                None
            }
        },
        None => {
            eprintln!("no log file: the local data folder is unknown");
            None
        }
    };
    if log::set_logger(LOGGER.get_or_init(|| Logger { file })).is_ok() {
        let level = std::env::var("RUST_LOG")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(LevelFilter::Info)
            .max(LevelFilter::Info);
        log::set_max_level(level);
    }
}

/// The log file: `flag` when given, else `Handpic.log` in the native app's
/// own `<IDENTIFIER>.native` folder in the shape of Tauri's `app_log_dir`
/// (`<base>/<id>` on macOS, `base` being `~/Library/Logs`; `<base>/<id>/logs`
/// elsewhere, `base` being the local data dir); `None` when neither is known.
pub fn log_path(flag: Option<PathBuf>, base: Option<&Path>) -> Option<PathBuf> {
    flag.or_else(|| {
        base.map(|base| {
            let dir = base.join(format!("{IDENTIFIER}.native"));
            let dir = if cfg!(target_os = "macos") {
                dir
            } else {
                dir.join("logs")
            };
            dir.join(LOG_FILE)
        })
    })
}

fn line(secs: u64, target: &str, level: log::Level, args: &std::fmt::Arguments) -> String {
    let (days, rest) = (secs / 86_400, secs % 86_400);
    let (y, m, d) = civil(days as i64);
    format!(
        "[{y:04}-{m:02}-{d:02}][{:02}:{:02}:{:02}][{target}][{level}] {args}\n",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

/// The date `days` after 1970-01-01 (Howard Hinnant's `civil_from_days`).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, m, d)
}

/// The open file, its size kept in memory rather than read per line.
struct Sink {
    path: PathBuf,
    file: File,
    size: u64,
    cap: u64,
}

impl Sink {
    fn open(path: &Path, cap: u64) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let size = file.metadata()?.len();
        Ok(Self {
            path: path.to_path_buf(),
            file,
            size,
            cap,
        })
    }

    fn write(&mut self, line: &str) {
        let len = line.len() as u64;
        if self.size > 0 && self.size + len > self.cap {
            match File::create(&self.path) {
                Ok(file) => {
                    self.file = file;
                    self.size = 0;
                }
                Err(_) => return,
            }
        }
        if self.file.write_all(line.as_bytes()).is_ok() {
            self.size += len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LOG_FILE, Sink, line, log_path};
    use std::path::{Path, PathBuf};

    #[test]
    fn a_line_carries_the_utc_date_time_target_and_level() {
        assert_eq!(
            line(0, "riffle_native", log::Level::Info, &format_args!("a b")),
            "[1970-01-01][00:00:00][riffle_native][INFO] a b\n"
        );
        assert_eq!(
            line(
                1_709_251_199,
                "riffle_backend::index",
                log::Level::Warn,
                &format_args!("x={}", 1)
            ),
            "[2024-02-29][23:59:59][riffle_backend::index][WARN] x=1\n"
        );
        assert_eq!(
            line(1_794_618_123, "t", log::Level::Error, &format_args!("m")),
            "[2026-11-14][01:02:03][t][ERROR] m\n"
        );
    }

    #[test]
    fn the_flag_beats_the_default() {
        assert_eq!(
            log_path(Some(PathBuf::from("D:/t/a.log")), Some(Path::new("C:/d"))),
            Some(PathBuf::from("D:/t/a.log"))
        );
        assert_eq!(log_path(None, None), None);
    }

    #[test]
    fn the_default_is_the_native_folder_in_the_tauri_log_dir_shape() {
        let dir = Path::new("C:/d").join("com.minodisk.handpic.native");
        let dir = if cfg!(target_os = "macos") {
            dir
        } else {
            dir.join("logs")
        };
        assert_eq!(
            log_path(None, Some(Path::new("C:/d"))),
            Some(dir.join(LOG_FILE))
        );
    }

    #[test]
    fn the_file_starts_anew_when_the_next_line_would_pass_the_cap() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join(LOG_FILE);
        let mut sink = Sink::open(&path, 11).unwrap();
        sink.write("12345\n");
        sink.write("abcd\n");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "12345\nabcd\n");
        sink.write("x\n");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x\n");
        drop(sink);
        let mut sink = Sink::open(&path, 11).unwrap();
        sink.write("0123456\n");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "x\n0123456\n");
        sink.write("y\n");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "y\n");
    }
}
