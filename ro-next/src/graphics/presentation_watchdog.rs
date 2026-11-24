//! Watchdog for a stalled presentation path.
//!
//! NVIDIA's Vulkan Wayland WSI can block forever inside the swapchain image
//! acquire (`drmSyncobjTimelineWait`) when the compositor stops handing
//! swapchain images back to the client. The blocking call cannot be interrupted
//! from Rust, so a helper thread observes the acquire and, when it does not
//! finish, restarts the client on X11/XWayland instead of leaving an
//! unresponsive window behind.
//!
//! The watchdog is only useful as a last resort: while the window is in the
//! background the client stops presenting frames entirely (Wayland does not
//! report occlusion to clients), which avoids the stall in the first place. See
//! `Client::should_pause_presentation`.

use std::process::Command;
use std::sync::{Mutex, Once};
use std::time::{Duration, Instant};

use crate::logging::{log_error, log_warn};

/// How long a swapchain image acquire may block before the client is considered
/// stuck.
const STALL_TIMEOUT: Duration = Duration::from_secs(10);
/// How often the watchdog checks the acquire timestamp.
const POLL_INTERVAL: Duration = Duration::from_millis(250);

/// Disables the automatic X11 fallback; the client then only logs the stall and
/// exits.
pub const DISABLE_FALLBACK_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_NO_X11_FALLBACK";
/// Set for the relaunched client so that the watchdog never falls back in a
/// loop.
pub const FALLBACK_ACTIVE_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_X11_FALLBACK_ACTIVE";
/// Test hook: pretend the swapchain acquire blocks for this many milliseconds.
pub const TEST_STALL_ENVIRONMENT_VARIABLE: &str = "RO_NEXT_TEST_ACQUIRE_STALL_MS";

static ACQUIRE_STARTED_AT: Mutex<Option<Instant>> = Mutex::new(None);
static WATCHDOG: Once = Once::new();

fn set_acquire_started_at(started_at: Option<Instant>) {
    if let Ok(mut guard) = ACQUIRE_STARTED_AT.lock() {
        *guard = started_at;
    }
}

/// Marks the beginning of a blocking swapchain image acquisition.
pub fn begin_acquire() {
    set_acquire_started_at(Some(Instant::now()));
}

/// Marks the end of a swapchain image acquisition.
pub fn end_acquire() {
    set_acquire_started_at(None);
}

/// Sleeps for the configured synthetic stall (only used by the test hook).
pub fn simulate_stall_if_configured() {
    let Some(milliseconds) = std::env::var(TEST_STALL_ENVIRONMENT_VARIABLE)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    else {
        return;
    };

    std::thread::sleep(Duration::from_millis(milliseconds));
}

/// Starts the watchdog thread (only the first call has an effect).
///
/// `backend_is_wayland` decides whether the client is restarted on X11/XWayland
/// when the acquisition stalls. On X11 a stalled acquire cannot be recovered
/// either, but there is no second backend to fall back to, so the client logs
/// the problem and exits instead of hanging forever.
pub fn start(backend_is_wayland: bool) {
    WATCHDOG.call_once(move || {
        let spawn_result = std::thread::Builder::new().name("presentation-watchdog".to_owned()).spawn(move || {
            loop {
                std::thread::sleep(POLL_INTERVAL);

                let started_at = match ACQUIRE_STARTED_AT.lock() {
                    Ok(guard) => *guard,
                    Err(poisoned) => *poisoned.into_inner(),
                };

                let Some(started_at) = started_at else {
                    continue;
                };

                let elapsed = started_at.elapsed();

                if elapsed < STALL_TIMEOUT {
                    continue;
                }

                // The watchdog only fires once.
                set_acquire_started_at(None);

                handle_stall(elapsed, backend_is_wayland);
            }
        });

        if let Err(error) = spawn_result {
            log_error!("[surface] failed to start the presentation watchdog: {error}");
        }
    });
}

fn handle_stall(elapsed: Duration, backend_is_wayland: bool) {
    log_error!(
        "[surface] the swapchain image acquisition is blocked for {:.1}s, the compositor stopped releasing images",
        elapsed.as_secs_f64(),
    );

    let fallback_disabled = environment_flag(DISABLE_FALLBACK_ENVIRONMENT_VARIABLE);
    let already_fallback = std::env::var_os(FALLBACK_ACTIVE_ENVIRONMENT_VARIABLE).is_some();

    if backend_is_wayland && !fallback_disabled && !already_fallback {
        match relaunch_on_x11() {
            Ok(_) => {
                log_warn!(
                    "[surface] restarting the client on X11/XWayland (RO_NEXT_X11=1) to recover from the native Wayland presentation \
                     stall"
                );
                std::process::exit(0);
            }
            Err(error) => {
                log_error!("[surface] failed to restart the client on X11/XWayland: {error}");
            }
        }
    }

    if backend_is_wayland {
        log_error!("[surface] relaunch with RO_NEXT_X11=1 to run the client on X11/XWayland");
    } else {
        log_error!("[surface] the presentation path is stuck; the client has to be restarted");
    }

    std::process::exit(1);
}

fn relaunch_on_x11() -> std::io::Result<()> {
    let executable = std::env::current_exe()?;

    let mut command = Command::new(executable);
    command.args(std::env::args_os().skip(1));
    command.env_remove("RO_NEXT_WAYLAND");
    command.env_remove(TEST_STALL_ENVIRONMENT_VARIABLE);
    // XWayland needs the display number. The Wayland session exports DISPLAY, but
    // make sure the fallback still works when the client was launched with the
    // environment scrubbed.
    let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_owned());
    command.env("DISPLAY", display);
    command.env("RO_NEXT_X11", "1");
    command.env("WINIT_UNIX_BACKEND", "x11");
    command.env(FALLBACK_ACTIVE_ENVIRONMENT_VARIABLE, "1");
    command.spawn()?;

    Ok(())
}

fn environment_flag(variable: &str) -> bool {
    std::env::var(variable).is_ok_and(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
}

/// Logs one line at startup when the client was restarted by the watchdog.
pub fn log_fallback_startup_line() {
    if std::env::var_os(FALLBACK_ACTIVE_ENVIRONMENT_VARIABLE).is_none() {
        return;
    }

    log_warn!(
        "[surface] this client was restarted because native Wayland presentation stalled (RO_NEXT_X11=1 is active, set \
         RO_NEXT_WAYLAND=1 to retry native Wayland)"
    );
}
