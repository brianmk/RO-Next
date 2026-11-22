//! RO-Next client entry point.
//!
//! Windows builds use the GUI subsystem so launching the client from the
//! patcher does not open a console window; stderr logging still works when a
//! console or redirection is attached.
#![cfg_attr(windows, windows_subsystem = "windows")]

use clap::Parser;
use korangar::Client;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Arguments {
    /// Synchronize the asset cache and exit.
    #[arg(long)]
    sync_cache: bool,

    /// Validate the item stats data and exit.
    ///
    /// Without a path the compiled fallback is validated; with a path that data
    /// file is validated (for example an archive override of
    /// `data\client\item_stats.ron`). Exits 0 when valid, 1 with the schema
    /// errors otherwise.
    #[arg(long, value_name = "PATH", num_args = 0..=1)]
    check_items: Option<Option<std::path::PathBuf>>,

    /// Validate the unified skill effect descriptor data and exit.
    ///
    /// Without a path the compiled fallback is validated; with a path that data
    /// file is validated (for example an archive override of
    /// `data\client\skill_effects.ron`). Exits 0 when valid, 1 with the schema
    /// errors otherwise.
    #[arg(long, value_name = "PATH", num_args = 0..=1)]
    check_skill_effects: Option<Option<std::path::PathBuf>>,

    /// Validate the data-driven protocol result-code message table and exit.
    ///
    /// Without a path the compiled fallback is validated; with a path that data
    /// file is validated (for example an archive override of
    /// `data\client\messages.ron`). Exits 0 when valid, 1 with the schema
    /// errors otherwise.
    #[arg(long = "check-messages", value_name = "PATH", num_args = 0..=1)]
    check_messages: Option<Option<std::path::PathBuf>>,

    /// Increase log verbosity: -v info, -vv debug, -vvv trace.
    ///
    /// Logs are written to stderr; the launcher scripts redirect them to
    /// logs/korangar-client-*.log. The environment variable KORANGAR_LOG_LEVEL
    /// (0-3) can be used instead when arguments cannot be passed.
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count, help = "Increase log verbosity (-v info, -vv debug, -vvv trace)")]
    verbose: u8,
}

fn main() {
    let arguments = Arguments::parse();
    let verbosity = korangar::logging::resolve_verbosity(arguments.verbose);

    korangar::logging::init(verbosity);
    korangar::crash::install();

    // Packet layout validation is log-only in release; route the (deduplicated)
    // mismatches through the client logging facade so they end up in the log
    // file with the normal level prefixes.
    ragnarok_packets::validation::set_reporter(report_packet_layout_mismatch);

    // Must happen before the display backend is initialized (see below).
    sanitize_environment();

    // If the presentation watchdog restarted this client on X11/XWayland, print one
    // clear line so it is obvious why the backend changed.
    korangar::log_presentation_fallback_startup_line();

    if let Some(path) = arguments.check_items {
        match korangar::check_item_stats(path.as_deref()) {
            Ok(count) => {
                println!("item stats: {count} item rows are valid");
                return;
            }
            Err(error) => {
                eprintln!("item stats: invalid data: {error}");
                std::process::exit(1);
            }
        }
    }

    if let Some(path) = arguments.check_skill_effects {
        match korangar::check_skill_effects(path.as_deref()) {
            Ok(count) => {
                println!("skill effects: {count} (skill, phase) descriptors are valid");
                return;
            }
            Err(error) => {
                eprintln!("skill effects: invalid data: {error}");
                std::process::exit(1);
            }
        }
    }

    if let Some(path) = arguments.check_messages {
        match korangar::check_messages(path.as_deref()) {
            Ok(count) => {
                println!("messages: {count} protocol result messages are valid");
                return;
            }
            Err(error) => {
                eprintln!("messages: invalid data: {error}");
                std::process::exit(1);
            }
        }
    }

    korangar::logging::log_info!(
        "RO-Next {} starting (log verbosity {}, {} command line -v flags)",
        env!("CARGO_PKG_VERSION"),
        verbosity,
        arguments.verbose,
    );

    if let Some(mut client) = Client::init(arguments.sync_cache) {
        client.run();
    }

    korangar::logging::log_info!("RO-Next exited normally");
}

/// Removes environment variables that would make the display backend detection
/// fail before the client can even try to start.
///
/// Some launchers export an empty `WAYLAND_SOCKET` variable. The Wayland client
/// library treats a set `WAYLAND_SOCKET` as a file descriptor and fails with
/// "InvalidFd" if it is empty or not a valid file descriptor, even when
/// `WAYLAND_DISPLAY` points to a perfectly usable compositor. The client cannot
/// recover from that once the Wayland connection is attempted, so the variable
/// is sanitized before anything else looks at the environment.
fn sanitize_environment() {
    // WAYLAND_SOCKET is a Linux/libwayland concept; Windows has no such
    // variable and no POSIX fcntl(2), so this is a no-op there.
    #[cfg(unix)]
    {
        let Ok(value) = std::env::var("WAYLAND_SOCKET") else {
            return;
        };

        let valid = value.trim().parse::<libc::c_int>().ok().is_some_and(|file_descriptor| {
            // SAFETY: `fcntl` with `F_GETFD` only queries the file descriptor
            // table and does not dereference any pointer.
            unsafe { libc::fcntl(file_descriptor, libc::F_GETFD) != -1 }
        });

        if !valid {
            // SAFETY: This runs at the very start of `main`, before any thread or
            // library is started, so there is no concurrent access to the
            // environment.
            unsafe { std::env::remove_var("WAYLAND_SOCKET") };

            korangar::logging::log_warn!("[display] ignoring invalid WAYLAND_SOCKET environment variable");
        }
    }
}

/// Reports a packet layout mismatch at warn (fixed-size) or debug
/// (variable-length) level.
///
/// The check itself is log-only in release builds; tests and
/// `KORANGAR_PACKET_STRICT=1` turn it into an error instead.
fn report_packet_layout_mismatch(mismatch: &ragnarok_packets::validation::SizeMismatch) {
    match mismatch.check {
        ragnarok_packets::validation::LengthCheck::Fixed { .. } => korangar::logging::log_warn!(
            "packets: layout mismatch for {} (0x{:04X}): declared {} bytes, consumed {} bytes",
            mismatch.packet_type,
            mismatch.header.0,
            mismatch.check.claim(),
            mismatch.check.actual(),
        ),
        ragnarok_packets::validation::LengthCheck::Variable { .. } => korangar::logging::log_debug!(
            "packets: layout mismatch for {} (0x{:04X}): declared {} bytes, consumed {} bytes",
            mismatch.packet_type,
            mismatch.header.0,
            mismatch.check.claim(),
            mismatch.check.actual(),
        ),
    }
}
