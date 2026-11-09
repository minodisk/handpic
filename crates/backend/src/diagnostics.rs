//! Failures that would otherwise leave no trace in `Handpic.log`: Rust panics,
//! the frontend's uncaught errors and broken internal assumptions, each
//! written under a fixed, greppable prefix.

use std::backtrace::{Backtrace, BacktraceStatus};
use std::panic::Location;

/// Log every panic, on any thread, as one `panic:` line before the default
/// hook prints it to stderr. Panics the core catches and turns into an `Err`
/// (corrupt-file decodes) are logged too, so a `panic:` line is a crash only
/// when no recovery follows. It is installed from `.setup()`, after the log
/// plugin, so a panic before `setup` only reaches stderr.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| info.payload().downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<non-string payload>");
        let line = panic_line(std::thread::current().name(), info.location(), payload);
        let backtrace = Backtrace::capture();
        if backtrace.status() == BacktraceStatus::Captured {
            log::error!("{line}\n{backtrace}");
        } else {
            log::error!("{line}");
        }
        previous(info);
    }));
}

fn panic_line(thread: Option<&str>, location: Option<&Location>, payload: &str) -> String {
    let location = location.map_or_else(
        || "unknown".to_string(),
        |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
    );
    format!(
        "panic: thread={} location={location} payload={}",
        thread.unwrap_or("unnamed"),
        payload.replace(['\r', '\n'], " ")
    )
}

pub fn invariant_line(name: &str, details: std::fmt::Arguments) -> String {
    format!("invariant: {name}: {details}")
}

/// Log `invariant: <name>: <details>` at `warn` when `cond` is false. Only the
/// condition is evaluated on the happy path; it never changes control flow.
#[macro_export]
macro_rules! invariant {
    ($cond:expr, $name:expr, $($details:tt)+) => {
        if !$cond {
            log::warn!(
                "{}",
                $crate::diagnostics::invariant_line($name, format_args!($($details)+))
            );
        }
    };
}
pub use crate::invariant;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_line_names_the_thread_location_and_payload() {
        let location = Location::caller();
        let line = panic_line(Some("scan"), Some(location), "boom");
        assert_eq!(
            line,
            format!(
                "panic: thread=scan location={}:{}:{} payload=boom",
                location.file(),
                location.line(),
                location.column()
            )
        );
    }

    #[test]
    fn panic_line_without_a_thread_name_or_location() {
        assert_eq!(
            panic_line(None, None, "<non-string payload>"),
            "panic: thread=unnamed location=unknown payload=<non-string payload>"
        );
    }

    #[test]
    fn panic_line_collapses_newlines_in_the_payload() {
        assert_eq!(
            panic_line(Some("main"), None, "a\nb\r\nc"),
            "panic: thread=main location=unknown payload=a b  c"
        );
    }

    #[test]
    fn invariant_line_names_the_check_and_its_details() {
        assert_eq!(
            invariant_line(
                "scan progress within total",
                format_args!("done={} total={}", 3, 2)
            ),
            "invariant: scan progress within total: done=3 total=2"
        );
    }
}
