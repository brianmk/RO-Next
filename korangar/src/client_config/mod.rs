//! Data-driven client configuration files.
//!
//! The client ships a few small configuration values as data instead of Rust
//! constants, so a server (or a different client distribution) can override
//! them inside its own archive without rebuilding the client:
//!
//! * [`world_config`] (`data\client\world_config.ron`) — login-screen/default
//!   map, start-camera focus point, quest-icon texture template and colours,
//! * [`ui_audio`] (`data\client\ui_audio.ron`) — menu background music, menu
//!   click sound and teleport-vanish sound effect,
//! * [`debug_maps`] (`data\client\debug_maps.ron`) — the debug-only map warp
//!   list of the Maps window.
//!
//! Every file follows the same contract used by the other data-driven systems
//! (`skills.ron`, `jobs.ron`, ...):
//!
//! * loaded through [`GameFileLoader`] so an archive override wins,
//! * embedded with `include_str!` as the compiled fallback,
//! * `deny_unknown_fields` + explicit validation, so a typo is loud instead of
//!   silently ignored,
//! * a malformed override is logged and the compiled fallback is used,
//! * `korangar --check-world`, `--check-ui-audio` and `--check-debug-maps`
//!   validate a file without starting a window.
//!
//! [`GameFileLoader`]: crate::loaders::GameFileLoader

mod debug_maps;
mod ui_audio;
mod world_config;

#[cfg(feature = "debug")]
pub(crate) use self::debug_maps::{load as load_debug_maps, warps};
pub(crate) use self::debug_maps::{validate as validate_debug_maps, validate_compiled as validate_compiled_debug_maps};
pub use self::ui_audio::UiAudioSummary;
pub(crate) use self::ui_audio::{
    background_music, load as load_ui_audio, main_menu_click_sound, teleport_vanish_sound, validate as validate_ui_audio,
    validate_compiled as validate_compiled_ui_audio,
};
pub use self::world_config::WorldConfigSummary;
pub(crate) use self::world_config::{
    default_map, load as load_world_config, quest_icon_color, quest_icon_texture, start_camera_focus, validate as validate_world_config,
    validate_compiled as validate_compiled_world_config,
};
use crate::loaders::GameFileLoader;

/// Loads every client configuration data file from the client archive (or the
/// compiled fallbacks).
///
/// Called once at start-up; the individual loaders log whether the archive
/// override or the compiled fallback was used.
pub(crate) fn load(game_file_loader: &GameFileLoader) {
    load_world_config(game_file_loader);
    load_ui_audio(game_file_loader);
    // The debug map list is only consumed by the debug-only Maps window, so it
    // is not loaded (and not shipped in the archive override path) in release
    // builds.
    #[cfg(feature = "debug")]
    load_debug_maps(game_file_loader);
}
