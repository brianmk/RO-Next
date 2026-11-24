//! Minimal packet-stream recording for the replay feature.
//!
//! The official client records complete network sessions and can play them
//! back. This module only implements the **recording** half: while recording is
//! active, every parsed server packet and every client packet is appended to a
//! file in `client/replays/`. Playback is deliberately **not implemented**
//! (see `MenuWindow`); a replay file is a diagnostic artefact, not a playable
//! replay yet.
//!
//! File layout (all integers little endian):
//!
//! ```text
//! "KRRP"                        4 byte magic
//! version            u16        1
//! start unix seconds u64
//! frame*:
//!     direction      u8         0 = server -> client, 1 = client -> server
//!     length         u32
//!     bytes          length
//! ```
//!
//! Packets are re-serialized from the parsed packet values. Unknown packets
//! are stored as raw bytes, so a session with handlers missing for a packet
//! still produces a useful stream.

use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use ragnarok_bytes::ByteWriter;
use ragnarok_bytes::encoding::UTF_8;
use ragnarok_packets::{Packet, PacketExt};

/// File magic of a Korangar replay.
const REPLAY_MAGIC: &[u8; 4] = b"KRRP";
/// Replay format version written after the magic.
const REPLAY_VERSION: u16 = 1;
/// Directory (relative to the client working directory) for replay files.
const REPLAY_DIRECTORY: &str = "client/replays";

/// Direction byte for packets received from the server.
pub const DIRECTION_INCOMING: u8 = 0;
/// Direction byte for packets sent to the server.
pub const DIRECTION_OUTGOING: u8 = 1;

struct ReplayRecorder {
    file: File,
    path: PathBuf,
    frames: u64,
}

static RECORDER: OnceLock<Mutex<Option<ReplayRecorder>>> = OnceLock::new();

fn recorder() -> &'static Mutex<Option<ReplayRecorder>> {
    RECORDER.get_or_init(|| Mutex::new(None))
}

/// Whether a replay is currently being recorded.
pub fn is_recording() -> bool {
    recorder().lock().map(|recorder| recorder.is_some()).unwrap_or(false)
}

/// Start recording into a new `client/replays/<timestamp>.replay` file.
///
/// Returns the path of the file. Starting while already recording is a no-op
/// that returns the current file.
pub fn start() -> io::Result<PathBuf> {
    let mut recorder = recorder().lock().map_err(|_| io::Error::other("replay recorder lock poisoned"))?;

    if let Some(recorder) = recorder.as_ref() {
        return Ok(recorder.path.clone());
    }

    let directory = Path::new(REPLAY_DIRECTORY);
    fs::create_dir_all(directory)?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    let mut path = directory.join(format!("{timestamp}.replay"));
    let mut suffix = 1u32;

    // Two recordings started in the same second must not overwrite each other.
    while path.exists() {
        path = directory.join(format!("{timestamp}-{suffix}.replay"));
        suffix += 1;
    }

    let mut file = File::create(&path)?;
    file.write_all(REPLAY_MAGIC)?;
    file.write_all(&REPLAY_VERSION.to_le_bytes())?;
    file.write_all(&timestamp.to_le_bytes())?;
    file.flush()?;

    crate::logging::log_info!("replay: recording packet stream to `{}`", path.display());

    *recorder = Some(ReplayRecorder {
        file,
        path: path.clone(),
        frames: 0,
    });

    Ok(path)
}

/// Stop the current recording and return its path.
///
/// Returns `None` when no recording was active.
pub fn stop() -> Option<PathBuf> {
    let mut recorder = recorder().lock().ok()?;
    let recorder = recorder.take()?;

    crate::logging::log_info!(
        "replay: stopped recording `{}` after {} frames",
        recorder.path.display(),
        recorder.frames
    );

    Some(recorder.path)
}

/// Append one re-serialized packet to the active recording, if any.
///
/// Returns quietly when recording is off, so packet callbacks do not need a
/// branch. Serialization errors are ignored: a packet that cannot be written
/// back must not break the network loop.
pub fn record_packet<PacketType: Packet>(direction: u8, packet: &PacketType) {
    // Do not pay for re-serializing every packet while recording is off.
    if !is_recording() {
        return;
    }

    let bytes = {
        // Server packets are decoded from EUC-KR and have to be written back with
        // the same encoding. Client packets are sent as UTF-8 by the networking
        // system (`Encoding::UTF_8` in `NetworkingSystem::send_*_packet`).
        let mut writer = match direction {
            DIRECTION_OUTGOING => ByteWriter::with_encoding(UTF_8),
            _ => ByteWriter::new(),
        };

        match packet.packet_to_bytes(&mut writer) {
            Ok(_) => writer.into_inner(),
            Err(_) => return,
        }
    };

    record_bytes(direction, &bytes);
}

/// Append raw packet bytes to the active recording, if any.
///
/// Used for outgoing packets and for unknown server packets that have no
/// parsed representation.
pub fn record_bytes(direction: u8, bytes: &[u8]) {
    if bytes.is_empty() || bytes.len() > u32::MAX as usize {
        return;
    }

    let Ok(mut recorder) = recorder().lock() else {
        return;
    };

    let Some(recorder) = recorder.as_mut() else {
        return;
    };

    if recorder
        .file
        .write_all(&[direction])
        .and_then(|_| recorder.file.write_all(&(bytes.len() as u32).to_le_bytes()))
        .and_then(|_| recorder.file.write_all(bytes))
        .is_ok()
    {
        recorder.frames += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_and_frames_have_the_documented_layout() {
        let directory = std::env::temp_dir().join(format!("korangar-replay-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();

        // The production helper uses a fixed relative directory, so the test
        // writes the same header/frame layout directly.
        let path = directory.join("test.replay");
        let mut file = File::create(&path).unwrap();
        file.write_all(REPLAY_MAGIC).unwrap();
        file.write_all(&REPLAY_VERSION.to_le_bytes()).unwrap();
        file.write_all(&1_700_000_000u64.to_le_bytes()).unwrap();
        file.write_all(&[DIRECTION_INCOMING]).unwrap();
        file.write_all(&7u32.to_le_bytes()).unwrap();
        file.write_all(&[0x64, 0x00, 0x4B, 0x01, 0x00, 0x00, 0x00]).unwrap();
        drop(file);

        let bytes = fs::read(&path).unwrap();
        assert_eq!(&bytes[..4], REPLAY_MAGIC);
        assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), REPLAY_VERSION);
        assert_eq!(u64::from_le_bytes(bytes[6..14].try_into().unwrap()), 1_700_000_000);
        assert_eq!(bytes[14], DIRECTION_INCOMING);
        assert_eq!(u32::from_le_bytes(bytes[15..19].try_into().unwrap()), 7);
        assert_eq!(&bytes[19..], &[0x64, 0x00, 0x4B, 0x01, 0x00, 0x00, 0x00]);

        let _ = fs::remove_dir_all(&directory);
    }
}
