//! Wire-layout self-checks for packet structs (P0-2).
//!
//! The [`Packet`](crate::Packet) derive macro can compare the number of bytes a
//! packet struct actually consumed while parsing against the length the packet
//! itself declares:
//!
//! * variable-length packets (marked with `#[variable_length]`) declare their
//!   total size, including the two header bytes, in the first two payload
//!   bytes. `payload_from_bytes` has to consume exactly `declared - 2` payload
//!   bytes.
//! * fixed-size packets can opt into the same check with `#[packet_size(N)]`,
//!   where `N` is the full wire size including the header.
//!
//! The check is **log-only** by default so a layout drift can never break a
//! running client. Tests enable [`ValidationMode::Enforce`] (or set the
//! `KORANGAR_PACKET_STRICT` environment variable to `1`) to turn a mismatch
//! into a parse error instead.
//!
//! Packets whose layout is deliberately not self-consistent (for example
//! client-to-server packets with a trailing `NUL` that is not part of the
//! declared length) can opt out with `#[allow_packet_size_mismatch]`.

use std::cell::Cell;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, OnceLock};

use ragnarok_bytes::{ConversionError, ConversionResult};

use crate::PacketHeader;

/// How a layout mismatch is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationMode {
    /// Report mismatches (deduplicated per opcode) but keep parsing.
    ///
    /// This is the default in all builds, including release builds: a layout
    /// drift must never break a running client.
    LogOnly,
    /// Return a [`ConversionError`] from `payload_from_bytes` on a mismatch.
    ///
    /// Used by the contract tests and can be enabled at runtime with
    /// `KORANGAR_PACKET_STRICT=1`.
    Enforce,
}

const MODE_LOG_ONLY: u8 = 0;
const MODE_ENFORCE: u8 = 1;

/// The kind of layout check that failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LengthCheck {
    /// A variable-length packet declared `declared` bytes (header included)
    /// but the struct consumed `consumed` payload bytes.
    Variable { declared: usize, consumed: usize },
    /// A fixed-size packet is expected to be `expected` bytes on the wire
    /// (header included) but the struct consumed `consumed` payload bytes.
    Fixed { expected: usize, consumed: usize },
}

impl LengthCheck {
    /// Returns `true` when the amount of consumed bytes is consistent with the
    /// declared/expected length.
    pub fn is_consistent(self) -> bool {
        match self {
            LengthCheck::Variable { declared, consumed } => declared.saturating_sub(HEADER_SIZE) == consumed,
            LengthCheck::Fixed { expected, consumed } => expected.saturating_sub(HEADER_SIZE) == consumed,
        }
    }

    /// The wire size (header included) that the packet claims to have.
    pub fn claim(&self) -> usize {
        match self {
            LengthCheck::Variable { declared, .. } => *declared,
            LengthCheck::Fixed { expected, .. } => *expected,
        }
    }

    /// The wire size (header included) that the struct actually consumed.
    pub fn actual(&self) -> usize {
        match self {
            LengthCheck::Variable { consumed, .. } | LengthCheck::Fixed { consumed, .. } => consumed + HEADER_SIZE,
        }
    }
}

/// Two bytes of packet header.
pub const HEADER_SIZE: usize = 2;

/// A packet whose parsed size does not match its declared size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SizeMismatch {
    /// Header of the offending packet.
    pub header: PacketHeader,
    /// Rust type name of the packet struct.
    pub packet_type: &'static str,
    /// The failed check.
    pub check: LengthCheck,
}

/// Function pointer used to report mismatches to the application.
///
/// The app can install a reporter that routes the message through its own
/// logging facade (see `korangar/src/main.rs`).
pub type MismatchReporter = fn(&SizeMismatch);

fn env_enforce() -> bool {
    static ENFORCE: OnceLock<bool> = OnceLock::new();

    *ENFORCE.get_or_init(|| {
        std::env::var("KORANGAR_PACKET_STRICT")
            .map(|value| matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on"))
            .unwrap_or(false)
    })
}

fn mode_cell() -> &'static AtomicU8 {
    static MODE: AtomicU8 = AtomicU8::new(MODE_LOG_ONLY);

    &MODE
}

thread_local! {
    /// Per-thread override used by tests. The Rust test harness runs every test
    /// in its own thread (and the whole crate shares one process), so a
    /// process-wide mode would leak `Enforce` into unrelated tests.
    static THREAD_MODE: Cell<Option<ValidationMode>> = const { Cell::new(None) };
}

/// The current validation mode.
pub fn mode() -> ValidationMode {
    if let Some(mode) = THREAD_MODE.with(Cell::get) {
        return mode;
    }

    match mode_cell().load(Ordering::Relaxed) {
        MODE_ENFORCE => ValidationMode::Enforce,
        _ if env_enforce() => ValidationMode::Enforce,
        _ => ValidationMode::LogOnly,
    }
}

/// Set the validation mode for the current thread.
///
/// This is primarily intended for tests. A process-wide strict mode for the
/// whole application can be enabled with `KORANGAR_PACKET_STRICT=1`; release
/// builds otherwise keep the default [`ValidationMode::LogOnly`] so a layout
/// drift can never break a running client.
pub fn set_mode(mode: ValidationMode) {
    THREAD_MODE.with(|cell| cell.set(Some(mode)));
}

fn reporter_cell() -> &'static Mutex<Option<MismatchReporter>> {
    static REPORTER: OnceLock<Mutex<Option<MismatchReporter>>> = OnceLock::new();

    REPORTER.get_or_init(|| Mutex::new(None))
}

/// Install a reporter that receives every (deduplicated) layout mismatch.
pub fn set_reporter(reporter: MismatchReporter) {
    if let Ok(mut slot) = reporter_cell().lock() {
        *slot = Some(reporter);
    }
}

fn reported_cell() -> &'static Mutex<HashSet<u16>> {
    static REPORTED: OnceLock<Mutex<HashSet<u16>>> = OnceLock::new();

    REPORTED.get_or_init(|| Mutex::new(HashSet::new()))
}

fn report_once(mismatch: &SizeMismatch) {
    // A mismatching packet arrives on every server tick in the worst case, so
    // only the first violation per opcode is reported. That is enough to find
    // the offending layout without spamming the log.
    let is_first = reported_cell()
        .lock()
        .map(|mut reported| reported.insert(mismatch.header.0))
        .unwrap_or(false);

    if !is_first {
        return;
    }

    let reporter = reporter_cell().lock().ok().and_then(|reporter| *reporter);

    match reporter {
        Some(reporter) => reporter(mismatch),
        None => {
            // `ragnarok-packets` has no logging dependency; the default
            // reporter writes a single warning-like line to stderr so a
            // release client still surfaces layout drift in its log file.
            eprintln!(
                "[packets] WARN layout mismatch for {} (0x{:04X}): declared {} bytes, consumed {} bytes",
                mismatch.packet_type,
                mismatch.header.0,
                mismatch.check.claim(),
                mismatch.check.actual(),
            );
        }
    }
}

fn check(header: PacketHeader, packet_type: &'static str, check: LengthCheck) -> ConversionResult<()> {
    if check.is_consistent() {
        return Ok(());
    }

    report_once(&SizeMismatch {
        header,
        packet_type,
        check,
    });

    match mode() {
        ValidationMode::LogOnly => Ok(()),
        ValidationMode::Enforce => Err(ConversionError::from_message(format!(
            "packet layout mismatch for {packet_type} (0x{:04X}): declared {} bytes, consumed {} bytes (variable/expected payload \
             includes the 2 byte header)",
            header.0,
            check.claim(),
            check.actual(),
        ))),
    }
}

/// Check a variable-length packet: the declared length (header included) must
/// equal the bytes the struct consumed (header included).
pub fn check_variable_length(
    header: PacketHeader,
    packet_type: &'static str,
    declared_length: usize,
    consumed_payload_bytes: usize,
) -> ConversionResult<()> {
    check(header, packet_type, LengthCheck::Variable {
        declared: declared_length,
        consumed: consumed_payload_bytes,
    })
}

/// Check a fixed-size packet against its `#[packet_size(N)]` declaration.
pub fn check_fixed_length(
    header: PacketHeader,
    packet_type: &'static str,
    expected_wire_size: usize,
    consumed_payload_bytes: usize,
) -> ConversionResult<()> {
    check(header, packet_type, LengthCheck::Fixed {
        expected: expected_wire_size,
        consumed: consumed_payload_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consistent_checks_pass() {
        assert!(
            check_variable_length(PacketHeader(0x0001), "Packet", 12, 10).is_ok(),
            "declared 12 (header included) means 10 payload bytes"
        );
        assert!(check_fixed_length(PacketHeader(0x0002), "Packet", 8, 6).is_ok());
    }

    #[test]
    fn log_only_mode_never_fails() {
        set_mode(ValidationMode::LogOnly);

        assert!(check_variable_length(PacketHeader(0x1000), "Packet", 20, 10).is_ok());
        assert!(check_fixed_length(PacketHeader(0x1001), "Packet", 8, 2).is_ok());
    }

    #[test]
    fn enforce_mode_reports_mismatch() {
        set_mode(ValidationMode::Enforce);

        let error = check_variable_length(PacketHeader(0x2000), "Packet", 20, 10).unwrap_err();
        assert!(format!("{error:?}").contains("0x2000"));

        let error = check_fixed_length(PacketHeader(0x2001), "Packet", 8, 2).unwrap_err();
        assert!(format!("{error:?}").contains("0x2001"));

        set_mode(ValidationMode::LogOnly);
    }
}
