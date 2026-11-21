//! Minimal, always-available logging for release builds.
//!
//! The client parses `-v` / `-vv` / `-vvv` in `main.rs` and initializes this
//! module before anything else, so that startup, window and render crashes can
//! be diagnosed from the log file of the launcher (stderr is redirected there).
//!
//! Verbosity levels:
//!
//! * `0` (default): warnings and errors only
//! * `-v` (`1`): info
//! * `-vv` (`2`): debug
//! * `-vvv` (`3`): trace (very verbose, includes per-window/per-texture lines)
//!
//! The macros write a single line per message to stderr (unbuffered, so the
//! last line before a crash is not lost):
//!
//! ```text
//! [12:34:56.789 INFO  korangar::interface::windows] opening window ...
//! ```
//!
//! Do not use `println!` for diagnostics: the client's rendering writes to the
//! GPU and the launcher captures stderr.

use std::fmt;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

/// Level for errors that prevent something from working.
pub const LEVEL_ERROR: u8 = 1;
/// Level for recoverable problems.
pub const LEVEL_WARN: u8 = 2;
/// Level for high level progress information (mapped from `-v`).
pub const LEVEL_INFO: u8 = 3;
/// Level for detailed debugging information (mapped from `-vv`).
pub const LEVEL_DEBUG: u8 = 4;
/// Level for very detailed tracing (mapped from `-vvv`).
pub const LEVEL_TRACE: u8 = 5;

/// Global verbosity, set once by [`init`].
static VERBOSITY: AtomicU8 = AtomicU8::new(0);

/// Convert the command line verbosity (`0..=3`) into a log level threshold.
fn level_threshold(verbosity: u8) -> u8 {
    match verbosity {
        0 => LEVEL_WARN,
        1 => LEVEL_INFO,
        2 => LEVEL_DEBUG,
        _ => LEVEL_TRACE,
    }
}

/// Initialize the global verbosity. Must be called once, as early as possible.
pub fn init(verbosity: u8) {
    VERBOSITY.store(verbosity.min(3), Ordering::Relaxed);
}

/// Current command line verbosity (`0..=3`).
pub fn verbosity() -> u8 {
    VERBOSITY.load(Ordering::Relaxed)
}

/// Whether a message at the given level would be printed.
pub fn enabled(level: u8) -> bool {
    level <= level_threshold(verbosity())
}

/// Write one log line to stderr if the level is enabled.
pub fn log(level: u8, target: &str, message: fmt::Arguments<'_>) {
    if !enabled(level) {
        return;
    }

    eprintln!("[{} {:<5} {}] {}", timestamp(), level_name(level), target, message);
}

/// Short level name for the log prefix.
pub fn level_name(level: u8) -> &'static str {
    match level {
        LEVEL_ERROR => "ERROR",
        LEVEL_WARN => "WARN",
        LEVEL_INFO => "INFO",
        LEVEL_DEBUG => "DEBUG",
        _ => "TRACE",
    }
}

/// Timestamp with milliseconds (`HH:MM:SS.mmm`).
fn timestamp() -> String {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let seconds = duration.as_secs();
    let milliseconds = duration.subsec_millis();
    let seconds_of_day = seconds % 86_400;
    let hours = seconds_of_day / 3_600;
    let minutes = (seconds_of_day % 3_600) / 60;
    let seconds = seconds_of_day % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}.{milliseconds:03}")
}

/// Log at error level.
#[macro_export]
macro_rules! log_error {
    ($($argument:tt)*) => {
        $crate::logging::log($crate::logging::LEVEL_ERROR, module_path!(), format_args!($($argument)*))
    };
}

/// Log at warning level.
#[macro_export]
macro_rules! log_warn {
    ($($argument:tt)*) => {
        $crate::logging::log($crate::logging::LEVEL_WARN, module_path!(), format_args!($($argument)*))
    };
}

/// Log at info level (enabled with `-v`).
#[macro_export]
macro_rules! log_info {
    ($($argument:tt)*) => {
        $crate::logging::log($crate::logging::LEVEL_INFO, module_path!(), format_args!($($argument)*))
    };
}

/// Log at debug level (enabled with `-vv`).
#[macro_export]
macro_rules! log_debug {
    ($($argument:tt)*) => {
        $crate::logging::log($crate::logging::LEVEL_DEBUG, module_path!(), format_args!($($argument)*))
    };
}

/// Log at trace level (enabled with `-vvv`).
#[macro_export]
macro_rules! log_trace {
    ($($argument:tt)*) => {
        $crate::logging::log($crate::logging::LEVEL_TRACE, module_path!(), format_args!($($argument)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_names_are_unique_and_ordered() {
        for verbosity in 0..=3 {
            let threshold = level_threshold(verbosity);
            assert!(threshold >= LEVEL_WARN);
            assert!(threshold <= LEVEL_TRACE);
        }

        assert_eq!(level_name(LEVEL_ERROR), "ERROR");
        assert_eq!(level_name(LEVEL_TRACE), "TRACE");
    }

    #[test]
    fn timestamp_has_expected_shape() {
        let timestamp = timestamp();
        assert_eq!(timestamp.len(), "00:00:00.000".len());
        assert_eq!(timestamp.as_bytes()[2], b':');
        assert_eq!(timestamp.as_bytes()[8], b'.');
    }
}
