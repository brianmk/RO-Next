//! Logging facade for the client binary.
//!
//! The actual implementation lives in `ro-next-interface` so that the
//! interface crate can log as well (it is a separate crate and does not depend
//! on `ro-next`). Everything is re-exported here so that the rest of the
//! client can simply `use crate::logging::log_debug;`.

use std::env;

pub use ro_next_interface::logging::{
    LEVEL_DEBUG, LEVEL_ERROR, LEVEL_INFO, LEVEL_TRACE, LEVEL_WARN, enabled, init, level_name, log, verbosity,
};
pub use ro_next_interface::{log_debug, log_error, log_info, log_trace, log_warn};

/// Environment variable that can override the command line verbosity
/// (`RO_NEXT_LOG_LEVEL=0..3`). Useful when the client is started by a desktop
/// launcher that does not pass command line arguments.
pub const LOG_LEVEL_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_LOG_LEVEL";

/// Resolve the verbosity from `-v` flags (`0..=3`) and the environment
/// variable. The environment variable takes precedence (it is useful to debug
/// a launch that does not forward arguments).
pub fn resolve_verbosity(verbose_flags: u8) -> u8 {
    let environment = env::var(LOG_LEVEL_ENVIRONMENT_VARIABLE)
        .ok()
        .and_then(|value| value.parse::<u8>().ok())
        .map(|value| value.min(3));

    environment.unwrap_or(verbose_flags.min(3))
}

use std::collections::HashSet;
use std::fmt;
use std::sync::{Mutex, OnceLock};

/// Maximum number of unique asset-failure keys remembered for `warn_once`.
///
/// Beyond this limit the process keeps running but stops collecting keys, so a
/// pathological data set cannot grow this set without bound.
const MAX_TRACKED_WARNINGS: usize = 4096;

/// Decision of [`WarnOnceSet::decide`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarnOnceDecision {
    /// The key is new; the caller should log the message.
    Log,
    /// The key was already reported; the caller should stay silent.
    Suppress,
    /// The tracking limit is reached; log a single overflow summary.
    Overflow,
}

/// Remembers which asset-failure keys were already logged.
///
/// Missing or malformed game assets fail repeatedly: one missing sprite is
/// requested by every entity/item that references it. Logging every occurrence
/// would bury the rest of the log, so the client logs the first one per asset
/// and suppresses the duplicates. The set is pure data so it can be unit
/// tested without capturing stderr.
#[derive(Default)]
pub struct WarnOnceSet {
    seen: HashSet<String>,
    overflow_logged: bool,
}

impl WarnOnceSet {
    pub fn decide(&mut self, key: &str) -> WarnOnceDecision {
        if self.seen.contains(key) {
            return WarnOnceDecision::Suppress;
        }

        if self.seen.len() >= MAX_TRACKED_WARNINGS {
            if self.overflow_logged {
                return WarnOnceDecision::Suppress;
            }

            self.overflow_logged = true;
            return WarnOnceDecision::Overflow;
        }

        self.seen.insert(key.to_owned());
        WarnOnceDecision::Log
    }

    /// Number of distinct keys that were logged (test helper).
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.seen.len()
    }
}

/// Log a warning at most once per unique key.
///
/// `key` should identify the asset (usually its file path or a loader id) and
/// `target` is the usual log target, e.g. `module_path!()`.
///
/// This is the release-visible channel for recoverable asset failures: it is
/// active at the default verbosity (`WARN`), unlike the old `print_debug!`
/// calls which were compiled out in release builds.
pub fn warn_once(target: &str, key: &str, message: fmt::Arguments<'_>) {
    static WARN_ONCE: OnceLock<Mutex<WarnOnceSet>> = OnceLock::new();

    let tracker = WARN_ONCE.get_or_init(|| Mutex::new(WarnOnceSet::default()));
    let Ok(mut tracker) = tracker.lock() else {
        // A poisoned lock must not take the client down; log without dedup.
        log(LEVEL_WARN, target, message);
        return;
    };

    match tracker.decide(key) {
        WarnOnceDecision::Log => log(LEVEL_WARN, target, message),
        WarnOnceDecision::Suppress => {}
        WarnOnceDecision::Overflow => log(
            LEVEL_WARN,
            target,
            format_args!("more than {MAX_TRACKED_WARNINGS} unique asset failures; further warnings are suppressed"),
        ),
    }
}

/// Log a warn-level message once per unique `key`.
///
/// ```ignore
/// log_warn_once!("data\\sprite\\missing.spr", "failed to load sprite: {error:?}");
/// ```
#[macro_export]
macro_rules! log_warn_once {
    ($key:expr, $($argument:tt)*) => {
        $crate::logging::warn_once(module_path!(), $key, format_args!($($argument)*))
    };
}

#[cfg(test)]
mod tests {
    use super::{LEVEL_DEBUG, LEVEL_WARN, MAX_TRACKED_WARNINGS, WarnOnceDecision, WarnOnceSet, enabled};

    #[test]
    fn warn_level_is_visible_without_verbose_flags() {
        // Default verbosity (no `-v`) is what release testers run with: warn
        // messages (asset failures, ...) must be printed there while debug
        // messages stay hidden. This is the property the P0-6 asset logging
        // relies on.
        assert!(enabled(LEVEL_WARN));
        assert!(!enabled(LEVEL_DEBUG));
    }

    #[test]
    fn first_occurrence_is_logged_and_duplicates_are_suppressed() {
        let mut set = WarnOnceSet::default();

        assert_eq!(set.decide("data\\sprite\\a.spr"), WarnOnceDecision::Log);
        assert_eq!(set.decide("data\\sprite\\a.spr"), WarnOnceDecision::Suppress);
        assert_eq!(set.decide("data\\sprite\\b.spr"), WarnOnceDecision::Log);
        assert_eq!(set.decide("data\\sprite\\a.spr"), WarnOnceDecision::Suppress);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn overflow_is_reported_once_and_then_suppresses() {
        let mut set = WarnOnceSet::default();

        for index in 0..MAX_TRACKED_WARNINGS {
            assert_eq!(set.decide(&format!("asset-{index}")), WarnOnceDecision::Log);
        }

        assert_eq!(set.decide("one-too-many"), WarnOnceDecision::Overflow);
        assert_eq!(set.decide("one-too-many"), WarnOnceDecision::Suppress);
        assert_eq!(set.decide("another-one"), WarnOnceDecision::Suppress);
        assert_eq!(set.len(), MAX_TRACKED_WARNINGS);
    }
}
