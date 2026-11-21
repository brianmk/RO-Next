//! Crash diagnostics for release builds.
//!
//! Two independent mechanisms make a crash leave usable diagnostics in the log
//! file of the launcher (stderr is redirected to `logs/korangar-client-*.log`):
//!
//! * a panic hook that prints the panic message plus a full backtrace
//!   (`RUST_BACKTRACE` is not required),
//! * signal handlers for `SIGSEGV`/`SIGABRT`/`SIGBUS`/`SIGILL`/`SIGFPE` that
//!   print a marker and a glibc backtrace (for crashes that are not panics,
//!   e.g. inside a graphics driver), then restore the default handler and
//!   re-raise the signal so a core dump can still be produced.
//!
//! Everything is written directly to stderr; it is unbuffered, so the last log
//! line before the crash is not lost.

use std::sync::atomic::{AtomicBool, Ordering};

/// Install the panic hook and the fatal signal handlers.
pub fn install() {
    install_panic_hook();
    install_signal_handlers();
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::force_capture();

        eprintln!();
        eprintln!(
            "=== korangar: panic (log verbosity {}) ===",
            korangar_interface::logging::verbosity()
        );
        eprintln!("{panic_info}");
        eprintln!("=== backtrace (full, RUST_BACKTRACE not required) ===");
        eprintln!("{backtrace}");
        eprintln!("=== end of panic report ===");
    }));
}

/// Guards against recursive faults inside the signal handler.
static IN_FATAL_SIGNAL: AtomicBool = AtomicBool::new(false);

#[cfg(all(target_os = "linux", target_env = "gnu"))]
unsafe extern "C" {
    fn backtrace(buffer: *mut *mut libc::c_void, size: libc::c_int) -> libc::c_int;
    fn backtrace_symbols_fd(buffer: *const *mut libc::c_void, size: libc::c_int, fd: libc::c_int);
}

const FATAL_SIGNALS: [libc::c_int; 5] = [libc::SIGSEGV, libc::SIGABRT, libc::SIGBUS, libc::SIGILL, libc::SIGFPE];

fn install_signal_handlers() {
    #[cfg(unix)]
    for signal in FATAL_SIGNALS {
        // SAFETY: the handler only writes to stderr and re-raises the signal.
        unsafe {
            libc::signal(signal, fatal_signal_handler as *const () as libc::sighandler_t);
        }
    }
}

/// Async-signal-safe-ish fatal signal handler: write a marker, dump a backtrace
/// and re-raise the signal with the default handler so the process can core
/// dump like it would without the handler.
unsafe extern "C" fn fatal_signal_handler(signal: libc::c_int) {
    if IN_FATAL_SIGNAL.swap(true, Ordering::SeqCst) {
        // Recursive fault while handling the first one, bail out immediately.
        unsafe { libc::_exit(128 + signal) };
    }

    let header = b"\n=== korangar: fatal signal received ===\n";
    unsafe {
        libc::write(libc::STDERR_FILENO, header.as_ptr().cast(), header.len());
    }

    let signal_name = match signal {
        libc::SIGSEGV => b"signal: SIGSEGV (segmentation fault)\n".as_slice(),
        libc::SIGABRT => b"signal: SIGABRT (abort, e.g. failed assertion/validation)\n",
        libc::SIGBUS => b"signal: SIGBUS (bus error)\n",
        libc::SIGILL => b"signal: SIGILL (illegal instruction)\n",
        libc::SIGFPE => b"signal: SIGFPE (floating point exception)\n",
        _ => b"signal: unknown fatal signal\n",
    };
    unsafe {
        libc::write(libc::STDERR_FILENO, signal_name.as_ptr().cast(), signal_name.len());
    }

    unsafe {
        backtrace_to_stderr();
    }

    let footer = b"=== end of fatal signal report, re-raising signal for a core dump ===\n";
    unsafe {
        libc::write(libc::STDERR_FILENO, footer.as_ptr().cast(), footer.len());
        // Restore the default handler and re-raise so the kernel produces a
        // core dump (if enabled) and the exit status reflects the signal.
        libc::signal(signal, libc::SIG_DFL);
        libc::raise(signal);
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
unsafe fn backtrace_to_stderr() {
    const MAXIMUM_FRAMES: usize = 128;
    let mut frames: [*mut libc::c_void; MAXIMUM_FRAMES] = [std::ptr::null_mut(); MAXIMUM_FRAMES];

    // SAFETY: backtrace writes at most MAXIMUM_FRAMES pointers into the buffer.
    let frame_count = unsafe { backtrace(frames.as_mut_ptr(), MAXIMUM_FRAMES as libc::c_int) };

    if frame_count > 0 {
        // SAFETY: frame_count frames were initialized by backtrace above.
        unsafe {
            backtrace_symbols_fd(frames.as_ptr(), frame_count, libc::STDERR_FILENO);
        }
    }
}

#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
unsafe fn backtrace_to_stderr() {
    let message = b"(no signal backtrace support on this platform)\n";
    unsafe {
        libc::write(libc::STDERR_FILENO, message.as_ptr().cast(), message.len());
    }
}
