//! Logging facade for the client binary.
//!
//! The actual implementation lives in `korangar-interface` so that the
//! interface crate can log as well (it is a separate crate and does not depend
//! on `korangar`). Everything is re-exported here so that the rest of the
//! client can simply `use crate::logging::log_debug;`.

use std::env;

pub use korangar_interface::logging::{
    LEVEL_DEBUG, LEVEL_ERROR, LEVEL_INFO, LEVEL_TRACE, LEVEL_WARN, enabled, init, level_name, log, verbosity,
};
pub use korangar_interface::{log_debug, log_error, log_info, log_trace, log_warn};

/// Environment variable that can override the command line verbosity
/// (`KORANGAR_LOG_LEVEL=0..3`). Useful when the client is started by a desktop
/// launcher that does not pass command line arguments.
pub const LOG_LEVEL_ENVIRONMENT_VARIABLE: &str = "KORANGAR_LOG_LEVEL";

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
