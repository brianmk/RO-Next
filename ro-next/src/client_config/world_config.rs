//! Data-driven world / login-screen configuration.
//!
//! `archive/data/client/world_config.ron` (shipped inside the client archive,
//! loaded through [`GameFileLoader`] so a server can override it without
//! rebuilding the client; embedded with `include_str!` as the compiled
//! fallback) owns the world-facing client configuration that used to be
//! hard-coded constants:
//!
//! * **default_map** — the map the client loads for the login-screen background
//!   (`DEFAULT_MAP = "geffen"` in `lib.rs`),
//! * **start_camera_focus** — the point the start camera looks at while no
//!   character is on a map (`START_CAMERA_FOCUS_POINT` in `lib.rs`),
//! * **quest_icon** — the minimap quest-icon texture template
//!   (`유저인터페이스\minimap\quest_{effect}_{frame}.bmp`) and the four
//!   `QuestColor` RGB tints that were a hard-coded `format!`/`match` pair in
//!   [`crate::world::particles`].
//!
//! The table is validated on load (`deny_unknown_fields`): a malformed archive
//! override is rejected with a warning and the compiled fallback is used, so a
//! bad override can never leave the client without a default map or quest
//! icons. `ro-next --check-world [PATH]` is the matching CLI check.
//!
//! The previous values are the committed defaults, so a client without an
//! override behaves exactly like before; the difference is that a server (or a
//! different client distribution) can now change them with a data edit.

use std::sync::OnceLock;

use cgmath::Point3;
use ro_next_loaders::FileLoader;
use ragnarok_packets::QuestColor;
use serde::Deserialize;

use crate::graphics::Color;
use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\world_config.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../archive/data/client/world_config.ron");

/// Number of [`QuestColor`] variants, i.e. of colour rows the data file has to
/// define.
const QUEST_COLOR_COUNT: usize = 4;

// ---------------------------------------------------------------------------
// Data file schema.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorldConfigFile {
    #[allow(dead_code)]
    provenance: String,
    #[allow(dead_code)]
    version: u32,
    default_map: String,
    start_camera_focus: (f32, f32, f32),
    quest_icon: QuestIconFile,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestIconFile {
    /// Texture path template; must contain `{effect}` and `{frame}`.
    texture: String,
    /// Animation frame of the icon (the official client ships frames 1..3).
    frame: u8,
    /// One tint per [`QuestColor`].
    colors: Vec<QuestColorFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuestColorFile {
    color: QuestColorName,
    rgb: (u8, u8, u8),
}

/// The four quest-icon tints, named like [`QuestColor`] so the data file reads
/// like the protocol enum. Kept local because the packet enum does not
/// implement serde and the data file should not depend on packet traits.
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
enum QuestColorName {
    Yellow,
    Orange,
    Green,
    Purple,
}

impl QuestColorName {
    fn index(self) -> usize {
        match self {
            Self::Yellow => 0,
            Self::Orange => 1,
            Self::Green => 2,
            Self::Purple => 3,
        }
    }
}

// ---------------------------------------------------------------------------
// Loaded representation (leaked, process-lifetime data).
// ---------------------------------------------------------------------------

/// Validated world/login configuration.
pub(crate) struct WorldConfig {
    default_map: &'static str,
    start_camera_focus: Point3<f32>,
    quest_icon_texture: &'static str,
    quest_icon_frame: u8,
    /// Indexed by [`QuestColorName::index`].
    quest_icon_colors: [Color; QUEST_COLOR_COUNT],
}

/// Row counts of a validated data file (`--check-world`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldConfigSummary {
    /// Map loaded for the login-screen background.
    pub default_map: String,
    /// Number of quest-icon colour rows (one per `QuestColor`).
    pub quest_colors: usize,
}

static COMPILED: OnceLock<WorldConfig> = OnceLock::new();
static ARCHIVE: OnceLock<WorldConfig> = OnceLock::new();

fn compiled() -> &'static WorldConfig {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(config) => config,
        Err(error) => {
            crate::logging::log_warn!("world config: compiled fallback is invalid ({error})");
            WorldConfig::emergency()
        }
    })
}

fn registry() -> &'static WorldConfig {
    ARCHIVE.get().unwrap_or_else(compiled)
}

impl WorldConfig {
    /// Last-resort configuration used only when even the compiled data file
    /// cannot be parsed (a build/packaging bug; the data tests prove the
    /// committed file is valid).
    ///
    /// It carries the values the client used before they became data, so a
    /// broken embed does not block start-up. It is never read when the compiled
    /// fallback parses, which is the only way the client is shipped.
    fn emergency() -> Self {
        Self {
            default_map: "geffen",
            start_camera_focus: Point3::new(600.0, 0.0, 240.0),
            quest_icon_texture: "유저인터페이스\\minimap\\quest_{effect}_{frame}.bmp",
            quest_icon_frame: 1,
            quest_icon_colors: [
                Color::rgb_u8(200, 200, 30),
                Color::rgb_u8(200, 100, 30),
                Color::rgb_u8(30, 200, 30),
                Color::rgb_u8(200, 30, 200),
            ],
        }
    }
}

/// Loads the world configuration from the client archive (or the compiled
/// fallback).
///
/// Returns the number of quest-icon colour rows. A malformed data file is
/// reported and the compiled fallback is used instead.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let Some(content) = from_archive.as_deref() else {
        crate::logging::log_debug!(
            "world config: loaded the default map {:?} from the compiled fallback",
            compiled().default_map
        );
        return compiled().quest_icon_colors.len();
    };

    let config = match parse(content) {
        Ok(config) => {
            crate::logging::log_debug!(
                "world config: loaded the default map {:?} and {} quest colours from the archive",
                config.default_map,
                config.quest_icon_colors.len()
            );
            config
        }
        Err(error) => {
            crate::logging::log_warn!("world config: invalid {DATA_PATH} ({error}), using the compiled fallback");
            parse(FALLBACK).unwrap_or_else(|fallback_error| {
                crate::logging::log_warn!("world config: compiled fallback is invalid too ({fallback_error})");
                WorldConfig::emergency()
            })
        }
    };

    let count = config.quest_icon_colors.len();
    let _ = ARCHIVE.set(config);
    count
}

/// The map the client loads for the login-screen background.
pub(crate) fn default_map() -> &'static str {
    registry().default_map
}

/// The point the start camera looks at while no character is on a map.
pub(crate) fn start_camera_focus() -> Point3<f32> {
    registry().start_camera_focus
}

/// Texture path of the quest icon of `effect_id` (substitutes `{effect}` and
/// the configured animation frame).
pub(crate) fn quest_icon_texture(effect_id: usize) -> String {
    let config = registry();

    config
        .quest_icon_texture
        .replace("{effect}", &effect_id.to_string())
        .replace("{frame}", &config.quest_icon_frame.to_string())
}

/// The configured tint of a [`QuestColor`].
pub(crate) fn quest_icon_color(quest_color: QuestColor) -> Color {
    let index = match quest_color {
        QuestColor::Yellow => 0,
        QuestColor::Orange => 1,
        QuestColor::Green => 2,
        QuestColor::Purple => 3,
    };

    registry().quest_icon_colors[index]
}

/// Validates a document against the schema and returns its summary.
///
/// This is the `--check`-able entry point: the client binary exposes it as
/// `ro-next --check-world [PATH]` so CI can validate the committed data file
/// (or an archive override) without starting a window or a network connection.
pub(crate) fn validate(content: &str) -> Result<WorldConfigSummary, String> {
    parse(content).map(|config| WorldConfigSummary {
        default_map: config.default_map.to_owned(),
        quest_colors: config.quest_icon_colors.len(),
    })
}

/// Validates the compiled fallback (used by `--check-world` without a path and
/// by the tests).
pub(crate) fn validate_compiled() -> Result<WorldConfigSummary, String> {
    validate(FALLBACK)
}

// ---------------------------------------------------------------------------
// Parsing and validation.
// ---------------------------------------------------------------------------

fn parse(content: &str) -> Result<WorldConfig, String> {
    let file: WorldConfigFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;
    build(file)
}

fn build(file: WorldConfigFile) -> Result<WorldConfig, String> {
    validate_map_name(&file.default_map, "default_map")?;

    let (x, y, z) = file.start_camera_focus;
    for (name, value) in [("x", x), ("y", y), ("z", z)] {
        if !value.is_finite() {
            return Err(format!("start_camera_focus has a non-finite {name} component {value}"));
        }
    }

    let icon = file.quest_icon;
    if !icon.texture.contains("{effect}") || !icon.texture.contains("{frame}") {
        return Err("quest icon texture template has to contain {effect} and {frame}".to_owned());
    }
    let lowercase_texture = icon.texture.to_ascii_lowercase();
    if !lowercase_texture.ends_with(".bmp") && !lowercase_texture.ends_with(".tga") {
        return Err(format!(
            "quest icon texture template {:?} has to end in .bmp/.tga",
            icon.texture
        ));
    }
    if icon.frame == 0 {
        return Err("quest icon frame has to be >= 1".to_owned());
    }

    let mut colors = [Color::WHITE; QUEST_COLOR_COUNT];
    let mut seen = [false; QUEST_COLOR_COUNT];
    for row in icon.colors {
        let index = row.color.index();
        if seen[index] {
            return Err(format!("duplicate quest icon colour row for {:?}", row.color));
        }
        seen[index] = true;
        colors[index] = Color::rgb_u8(row.rgb.0, row.rgb.1, row.rgb.2);
    }
    if let Some(missing) = seen.iter().position(|seen| !seen) {
        return Err(format!("missing quest icon colour row for QuestColor index {missing}"));
    }

    Ok(WorldConfig {
        default_map: Box::leak(file.default_map.into_boxed_str()),
        start_camera_focus: Point3::new(x, y, z),
        quest_icon_texture: Box::leak(icon.texture.into_boxed_str()),
        quest_icon_frame: icon.frame,
        quest_icon_colors: colors,
    })
}

/// A map name is a file stem: non-empty, ASCII letters/digits/underscore (the
/// same characters the client's map paths use).
fn validate_map_name(value: &str, name: &str) -> Result<(), String> {
    if value.is_empty() {
        return Err(format!("{name} is empty"));
    }
    if !value.chars().all(|character| character.is_ascii_alphanumeric() || character == '_') {
        return Err(format!("{name} {value:?} contains characters that are not part of a map name"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use cgmath::Point3;
    use ragnarok_packets::QuestColor;

    use super::*;

    #[test]
    fn committed_data_has_the_previous_values() {
        let summary = validate_compiled().expect("the committed world config has to be valid");
        assert_eq!(summary.default_map, "geffen");
        assert_eq!(summary.quest_colors, QUEST_COLOR_COUNT);

        assert_eq!(default_map(), "geffen");
        assert_eq!(start_camera_focus(), Point3::new(600.0, 0.0, 240.0));
        assert_eq!(quest_icon_color(QuestColor::Yellow), Color::rgb_u8(200, 200, 30));
        assert_eq!(quest_icon_color(QuestColor::Orange), Color::rgb_u8(200, 100, 30));
        assert_eq!(quest_icon_color(QuestColor::Green), Color::rgb_u8(30, 200, 30));
        assert_eq!(quest_icon_color(QuestColor::Purple), Color::rgb_u8(200, 30, 200));
    }

    #[test]
    fn quest_icon_texture_template_is_expanded() {
        assert_eq!(
            quest_icon_texture(3),
            "유저인터페이스\\minimap\\quest_3_1.bmp",
            "the effect id and the configured frame have to be substituted"
        );
    }

    #[test]
    fn rejects_unknown_fields() {
        let tampered = FALLBACK.replace("    version: 1,", "    version: 1,\n    unknown_field: true,");
        let error = validate(&tampered).unwrap_err();
        assert!(error.contains("Unexpected field") || error.contains("unknown field"), "{error}");
    }

    #[test]
    fn rejects_bad_map_and_camera() {
        let empty_map = FALLBACK.replace("default_map: \"geffen\"", "default_map: \"\"");
        assert!(validate(&empty_map).unwrap_err().contains("default_map is empty"));

        let bad_map = FALLBACK.replace("default_map: \"geffen\"", "default_map: \"../geffen\"");
        assert!(validate(&bad_map).unwrap_err().contains("not part of a map name"));

        // Non-finite floats cannot be written in RON portably, so tamper with
        // the parsed schema struct directly.
        let mut file: WorldConfigFile = ron::from_str(FALLBACK).expect("fallback has to parse");
        file.start_camera_focus = (f32::INFINITY, 0.0, 240.0);
        assert!(
            build(file)
                .err()
                .expect("a non-finite camera has to be rejected")
                .contains("non-finite")
        );
    }

    #[test]
    fn rejects_bad_quest_icon() {
        let bad_template = FALLBACK.replace("quest_{effect}_{frame}.bmp", "quest.bmp");
        assert!(validate(&bad_template).unwrap_err().contains("{effect}"));

        let bad_suffix = FALLBACK.replace("quest_{effect}_{frame}.bmp", "quest_{effect}_{frame}.spr");
        assert!(validate(&bad_suffix).unwrap_err().contains(".bmp/.tga"));

        let bad_frame = FALLBACK.replace("frame: 1,", "frame: 0,");
        assert!(validate(&bad_frame).unwrap_err().contains("frame has to be >= 1"));

        let duplicate = FALLBACK.replace("(color: Orange, rgb: (200, 100, 30)),", "(color: Yellow, rgb: (200, 100, 30)),");
        assert!(validate(&duplicate).unwrap_err().contains("duplicate quest icon colour"));

        let missing = FALLBACK.replace("(color: Purple, rgb: (200, 30, 200)),", "");
        assert!(validate(&missing).unwrap_err().contains("missing quest icon colour"));
    }
}
