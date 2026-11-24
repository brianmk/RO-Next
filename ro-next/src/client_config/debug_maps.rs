//! Data-driven debug map warp list.
//!
//! `archive/data/client/debug_maps.ron` (shipped inside the client archive,
//! loaded through [`GameFileLoader`] so a server can override it without
//! rebuilding the client; embedded with `include_str!` as the compiled
//! fallback) owns the 23 map/coordinate pairs that used to be the
//! `MAP_WARPS` array in [`crate::interface::windows::maps`].
//!
//! This is a **debug-only convenience** list: the Maps window is a developer
//! window (the UI consistency lint lists it as a debug-only file), not shipped
//! user-facing content, so the data intentionally has no localization keys.
//! It stays in the archive as data so a server/tester can add a map without a
//! rebuild.
//!
//! Validation is strict (`deny_unknown_fields`, non-empty list, unique map
//! names, positive coordinates); a malformed archive override is logged and the
//! compiled fallback is used. `ro-next --check-debug-maps [PATH]` is the
//! matching CLI check.

// The Maps window (the only consumer of the runtime registry) is compiled with
// the `debug` feature only. The schema/validation half of this module is always
// compiled for `ro-next --check-debug-maps`, so in non-debug builds the
// runtime accessors are allowed to be unused instead of being gated out (which
// would reveal the module layout in every call site).
#![cfg_attr(not(feature = "debug"), allow(dead_code))]

use std::sync::OnceLock;

use ragnarok_packets::TilePosition;
use ro_next_loaders::FileLoader;
use serde::Deserialize;

use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\debug_maps.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../archive/data/client/debug_maps.ron");

// ---------------------------------------------------------------------------
// Data file schema.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DebugMapsFile {
    #[allow(dead_code)]
    provenance: String,
    #[allow(dead_code)]
    version: u32,
    maps: Vec<DebugMapFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DebugMapFile {
    map_name: String,
    x: u16,
    y: u16,
}

// ---------------------------------------------------------------------------
// Loaded representation (leaked, process-lifetime data).
// ---------------------------------------------------------------------------

/// One debug warp target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DebugWarp {
    map_name: &'static str,
    position: TilePosition,
}

impl DebugWarp {
    /// Map name of the warp target (e.g. `"geffen"`).
    pub(crate) fn map_name(&self) -> &'static str {
        self.map_name
    }

    /// Cell the debug warp drops the player on.
    pub(crate) fn position(&self) -> TilePosition {
        self.position
    }

    /// Map resource name used by the warp request (`"geffen.gat"`).
    pub(crate) fn map_file_name(&self) -> String {
        format!("{}.gat", self.map_name)
    }
}

static COMPILED: OnceLock<Vec<DebugWarp>> = OnceLock::new();
static ARCHIVE: OnceLock<Vec<DebugWarp>> = OnceLock::new();

fn compiled() -> &'static [DebugWarp] {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(warps) => warps,
        Err(error) => {
            crate::logging::log_warn!("debug maps: compiled fallback is invalid ({error})");
            Vec::new()
        }
    })
}

fn registry() -> &'static [DebugWarp] {
    ARCHIVE.get().map(Vec::as_slice).unwrap_or_else(|| compiled())
}

/// Loads the debug warp list from the client archive (or the compiled
/// fallback).
///
/// Returns the number of warp rows. A malformed data file is reported and the
/// compiled fallback is used instead.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let Some(content) = from_archive.as_deref() else {
        crate::logging::log_debug!("debug maps: loaded {} warps from the compiled fallback", compiled().len());
        return compiled().len();
    };

    let warps = match parse(content) {
        Ok(warps) => {
            crate::logging::log_debug!("debug maps: loaded {} warps from the archive", warps.len());
            warps
        }
        Err(error) => {
            crate::logging::log_warn!("debug maps: invalid {DATA_PATH} ({error}), using the compiled fallback");
            parse(FALLBACK).unwrap_or_else(|fallback_error| {
                crate::logging::log_warn!("debug maps: compiled fallback is invalid too ({fallback_error})");
                Vec::new()
            })
        }
    };

    let count = warps.len();
    let _ = ARCHIVE.set(warps);
    count
}

/// The configured debug warps, in data-file order (the window shows them in
/// that order).
pub(crate) fn warps() -> &'static [DebugWarp] {
    registry()
}

/// Validates a document against the schema and returns the number of warp
/// rows.
///
/// This is the `--check`-able entry point: the client binary exposes it as
/// `ro-next --check-debug-maps [PATH]` so CI can validate the committed data
/// file (or an archive override) without starting a window or a network
/// connection.
pub(crate) fn validate(content: &str) -> Result<usize, String> {
    parse(content).map(|warps| warps.len())
}

/// Validates the compiled fallback (used by `--check-debug-maps` without a path
/// and by the tests).
pub(crate) fn validate_compiled() -> Result<usize, String> {
    validate(FALLBACK)
}

// ---------------------------------------------------------------------------
// Parsing and validation.
// ---------------------------------------------------------------------------

fn parse(content: &str) -> Result<Vec<DebugWarp>, String> {
    let file: DebugMapsFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;
    build(file)
}

fn build(file: DebugMapsFile) -> Result<Vec<DebugWarp>, String> {
    if file.maps.is_empty() {
        return Err("the data file contains no debug warps".to_owned());
    }

    let mut warps: Vec<DebugWarp> = Vec::with_capacity(file.maps.len());
    for row in file.maps {
        let map_name = row.map_name;
        if map_name.is_empty() {
            return Err("a debug warp has an empty map name".to_owned());
        }
        if !map_name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        {
            return Err(format!("debug warp map name {map_name:?} is not a valid map name"));
        }
        if row.x == 0 || row.y == 0 {
            return Err(format!("debug warp {map_name:?} has a zero coordinate ({}, {})", row.x, row.y));
        }
        if warps.iter().any(|warp| warp.map_name == map_name) {
            return Err(format!("duplicate debug warp for map {map_name:?}"));
        }

        warps.push(DebugWarp {
            map_name: Box::leak(map_name.into_boxed_str()),
            position: TilePosition { x: row.x, y: row.y },
        });
    }

    Ok(warps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_data_has_the_previous_warp_list() {
        let count = validate_compiled().expect("the committed debug map data has to be valid");
        assert_eq!(count, 23);
        assert_eq!(warps().len(), 23);

        assert_eq!(warps()[0].map_name(), "geffen");
        assert_eq!(warps()[0].position(), TilePosition { x: 119, y: 59 });
        assert_eq!(warps()[0].map_file_name(), "geffen.gat");
        assert!(warps().iter().any(|warp| warp.map_name() == "mid_camp"));
        assert_eq!(
            warps().iter().find(|warp| warp.map_name() == "jawaii").map(DebugWarp::position),
            Some(TilePosition { x: 251, y: 132 })
        );
    }

    #[test]
    fn rejects_unknown_fields() {
        let tampered = FALLBACK.replace("    version: 1,", "    version: 1,\n    unknown_field: true,");
        let error = validate(&tampered).unwrap_err();
        assert!(error.contains("Unexpected field") || error.contains("unknown field"), "{error}");
    }

    #[test]
    fn rejects_bad_rows() {
        let empty = FALLBACK.replace(
            "        (map_name: \"mid_camp\", x: 180, y: 240),",
            "        (map_name: \"mid_camp\", x: 0, y: 240),",
        );
        assert!(validate(&empty).unwrap_err().contains("zero coordinate"));

        let duplicate = FALLBACK.replace(
            "        (map_name: \"mid_camp\", x: 180, y: 240),",
            "        (map_name: \"geffen\", x: 180, y: 240),",
        );
        assert!(validate(&duplicate).unwrap_err().contains("duplicate debug warp"));

        let bad_name = FALLBACK.replace(
            "(map_name: \"geffen\", x: 119, y: 59),",
            "(map_name: \"../geffen\", x: 119, y: 59),",
        );
        assert!(validate(&bad_name).unwrap_err().contains("not a valid map name"));
    }
}
