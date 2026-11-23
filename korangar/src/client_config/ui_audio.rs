//! Data-driven UI audio paths.
//!
//! `archive/data/client/ui_audio.ron` (shipped inside the client archive,
//! loaded through [`GameFileLoader`] so a server can override it without
//! rebuilding the client; embedded with `include_str!` as the compiled
//! fallback) owns the audio paths that used to be hard-coded constants in
//! `lib.rs`:
//!
//! * **ui_background_music** — the track of the screens outside a map (login,
//!   server and character selection; `UI_BACKGROUND_MUSIC`). `None` makes those
//!   screens silent, which is the documented behaviour of a server that does
//!   not want the menu music.
//! * **main_menu_click_sound** — the interface/button click sound
//!   (`MAIN_MENU_CLICK_SOUND_EFFECT`).
//! * **teleport_vanish_sound** — the sound played when another entity teleports
//!   away / logs out (`TELEPORT_VANISH_SOUND_EFFECT`).
//!
//! Fallbacks (documented and tested):
//!
//! * a missing archive file uses the compiled fallback,
//! * a malformed archive override is logged and the compiled fallback is used,
//! * `None`/empty paths disable the respective sound instead of triggering a
//!   load of the empty path (the audio engine would only log the failed load),
//! * the file resolution itself (archives, `data/mp3`, case-insensitive loose
//!   files) is unchanged and shared with the map music.
//!
//! `korangar --check-ui-audio [PATH]` is the matching CLI check.

use std::sync::OnceLock;

use korangar_loaders::FileLoader;
use serde::Deserialize;

use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\ui_audio.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../archive/data/client/ui_audio.ron");

/// Extensions the audio engine can decode (`korangar-audio`).
const SUPPORTED_EXTENSIONS: &[&str] = &[".mp3", ".wav", ".flac"];

// ---------------------------------------------------------------------------
// Data file schema.
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UiAudioFile {
    #[allow(dead_code)]
    provenance: String,
    #[allow(dead_code)]
    version: u32,
    /// Background music of the screens outside a map; `None` means silent.
    ui_background_music: Option<String>,
    main_menu_click_sound: String,
    teleport_vanish_sound: String,
}

// ---------------------------------------------------------------------------
// Loaded representation (leaked, process-lifetime data).
// ---------------------------------------------------------------------------

/// Validated UI audio paths.
pub(crate) struct UiAudio {
    ui_background_music: Option<&'static str>,
    main_menu_click_sound: Option<&'static str>,
    teleport_vanish_sound: Option<&'static str>,
}

/// Summary of a validated data file (`--check-ui-audio`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiAudioSummary {
    /// Background music of the menus, `None` when the menus are silent.
    pub background_music: Option<String>,
    /// Number of configured sound-effect paths.
    pub sound_effects: usize,
}

static COMPILED: OnceLock<UiAudio> = OnceLock::new();
static ARCHIVE: OnceLock<UiAudio> = OnceLock::new();

fn compiled() -> &'static UiAudio {
    COMPILED.get_or_init(|| match parse(FALLBACK) {
        Ok(audio) => audio,
        Err(error) => {
            crate::logging::log_warn!("ui audio: compiled fallback is invalid ({error})");
            UiAudio::emergency()
        }
    })
}

fn registry() -> &'static UiAudio {
    ARCHIVE.get().unwrap_or_else(compiled)
}

impl UiAudio {
    /// Last-resort configuration used only when even the compiled data file
    /// cannot be parsed (a build/packaging bug; the data tests prove the
    /// committed file is valid): silent menus and no UI sound effects, so the
    /// client still starts.
    fn emergency() -> Self {
        Self {
            ui_background_music: None,
            main_menu_click_sound: None,
            teleport_vanish_sound: None,
        }
    }
}

/// Loads the UI audio configuration from the client archive (or the compiled
/// fallback).
///
/// Returns the number of configured sound-effect paths. A malformed data file
/// is reported and the compiled fallback is used instead.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let Some(content) = from_archive.as_deref() else {
        crate::logging::log_debug!(
            "ui audio: loaded {} sound effects from the compiled fallback",
            compiled().sound_effect_count()
        );
        return compiled().sound_effect_count();
    };

    let audio = match parse(content) {
        Ok(audio) => {
            crate::logging::log_debug!(
                "ui audio: loaded menu music {:?} and {} sound effects from the archive",
                audio.ui_background_music,
                audio.sound_effect_count()
            );
            audio
        }
        Err(error) => {
            crate::logging::log_warn!("ui audio: invalid {DATA_PATH} ({error}), using the compiled fallback");
            parse(FALLBACK).unwrap_or_else(|fallback_error| {
                crate::logging::log_warn!("ui audio: compiled fallback is invalid too ({fallback_error})");
                UiAudio::emergency()
            })
        }
    };

    let count = audio.sound_effect_count();
    let _ = ARCHIVE.set(audio);
    count
}

impl UiAudio {
    fn sound_effect_count(&self) -> usize {
        let main_menu = if self.main_menu_click_sound.is_some() { 1 } else { 0 };
        let teleport_vanish = if self.teleport_vanish_sound.is_some() { 1 } else { 0 };

        main_menu + teleport_vanish
    }
}

/// Background music of the login/server/character selection screens; `None`
/// keeps those screens silent.
pub(crate) fn background_music() -> Option<&'static str> {
    registry().ui_background_music
}

/// Interface/button click sound; `None` disables it.
pub(crate) fn main_menu_click_sound() -> Option<&'static str> {
    registry().main_menu_click_sound
}

/// Sound played when another entity teleports away / logs out; `None` disables
/// it.
pub(crate) fn teleport_vanish_sound() -> Option<&'static str> {
    registry().teleport_vanish_sound
}

/// Validates a document against the schema and returns its summary.
///
/// This is the `--check`-able entry point: the client binary exposes it as
/// `korangar --check-ui-audio [PATH]` so CI can validate the committed data
/// file (or an archive override) without starting a window or a network
/// connection.
pub(crate) fn validate(content: &str) -> Result<UiAudioSummary, String> {
    parse(content).map(|audio| UiAudioSummary {
        background_music: audio.ui_background_music.map(str::to_owned),
        sound_effects: audio.sound_effect_count(),
    })
}

/// Validates the compiled fallback (used by `--check-ui-audio` without a path
/// and by the tests).
pub(crate) fn validate_compiled() -> Result<UiAudioSummary, String> {
    validate(FALLBACK)
}

// ---------------------------------------------------------------------------
// Parsing and validation.
// ---------------------------------------------------------------------------

fn parse(content: &str) -> Result<UiAudio, String> {
    let file: UiAudioFile = ron::from_str(content).map_err(|error| format!("schema error: {error}"))?;
    build(file)
}

fn build(file: UiAudioFile) -> Result<UiAudio, String> {
    let ui_background_music = match &file.ui_background_music {
        Some(path) => Some(leak(validate_path(path, "ui_background_music")?)),
        None => None,
    };
    let main_menu_click_sound = Some(leak(validate_path(&file.main_menu_click_sound, "main_menu_click_sound")?));
    let teleport_vanish_sound = Some(leak(validate_path(&file.teleport_vanish_sound, "teleport_vanish_sound")?));

    Ok(UiAudio {
        ui_background_music,
        main_menu_click_sound,
        teleport_vanish_sound,
    })
}

/// Leaks a validated path for the process-lifetime registry.
fn leak(path: String) -> &'static str {
    Box::leak(path.into_boxed_str())
}

/// Validates one audio path: non-empty string ending in a decodable extension.
fn validate_path(path: &str, name: &str) -> Result<String, String> {
    if path.is_empty() {
        return Err(format!("{name} is empty"));
    }
    let lowercase = path.to_ascii_lowercase();
    if !SUPPORTED_EXTENSIONS.iter().any(|extension| lowercase.ends_with(extension)) {
        return Err(format!("{name} {path:?} does not end in {}", SUPPORTED_EXTENSIONS.join("/")));
    }
    Ok(path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_data_has_the_previous_paths() {
        let summary = validate_compiled().expect("the committed UI audio data has to be valid");
        assert_eq!(summary.background_music.as_deref(), Some("bgm\\01.mp3"));
        assert_eq!(summary.sound_effects, 2);

        assert_eq!(background_music(), Some("bgm\\01.mp3"));
        assert_eq!(main_menu_click_sound(), Some("버튼소리.wav"));
        assert_eq!(teleport_vanish_sound(), Some("effect\\ef_teleportation.wav"));
    }

    #[test]
    fn none_music_is_valid_and_silent() {
        let silent = FALLBACK.replace("ui_background_music: Some(\"bgm\\\\01.mp3\")", "ui_background_music: None");
        let summary = validate(&silent).expect("None menu music has to be valid");
        assert_eq!(summary.background_music, None);
        assert_eq!(summary.sound_effects, 2);
    }

    #[test]
    fn rejects_unknown_fields() {
        let tampered = FALLBACK.replace("    version: 1,", "    version: 1,\n    unknown_field: true,");
        let error = validate(&tampered).unwrap_err();
        assert!(error.contains("Unexpected field") || error.contains("unknown field"), "{error}");
    }

    #[test]
    fn rejects_bad_paths() {
        let empty = FALLBACK.replace("main_menu_click_sound: \"버튼소리.wav\"", "main_menu_click_sound: \"\"");
        assert!(validate(&empty).unwrap_err().contains("main_menu_click_sound is empty"));

        let bad_extension = FALLBACK.replace(
            "teleport_vanish_sound: \"effect\\\\ef_teleportation.wav\"",
            "teleport_vanish_sound: \"effect\\\\ef_teleportation.txt\"",
        );
        assert!(validate(&bad_extension).unwrap_err().contains("teleport_vanish_sound"));
    }
}
