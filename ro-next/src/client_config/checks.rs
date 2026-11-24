//! `--check-*` validators for the data tables the client ships.
//!
//! Each one parses a table through the same loader the client uses and returns
//! a summary (or the error the CLI prints). `main.rs` exposes them as
//! `--check-items`, `--check-skills`, ... and CI runs the ones that need no
//! game files.

use crate::*;

/// Validates the unified skill effect descriptor data and returns the number of
/// `(skill, phase)` descriptors.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback
/// (`ro-next/archive/data/client/skill_effects.ron`) is validated. This is the
/// `--check`-able schema entry point: the client binary exposes it as `ro-next
/// --check-skill-effects [PATH]`, so CI can validate the committed data file
/// without starting a window, a network connection or the game archives.
pub fn check_skill_effects(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_skill_effect_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_skill_effects().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the item stats data (`archive/data/client/item_stats.ron`) and
/// returns the number of item rows.
pub fn check_item_stats(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_item_stats_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_item_stats().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven protocol result-code message table and returns the
/// number of message rows.
pub fn check_messages(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            ro_next_messages::validate(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => ro_next_messages::validate_compiled().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven status-effect table and returns the number of
/// status entries.
pub fn check_status_effects(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_status_effect_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_status_effects().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven status body-visual table and returns the number of
/// rows.
pub fn check_status_visuals(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_status_visual_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_status_visuals().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven job/appearance registry and returns the number of
/// job rows.
pub fn check_jobs(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::jobs::validate(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::jobs::validate_compiled().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the generated alias tables (`archive/data/client/aliases.ron`) and
/// returns the row counts.
///
/// The tables (job skill-tree fallbacks, monster sprite aliases, skill icon
/// aliases) are content data extracted from the client archives and the rAthena
/// server tree; see `tools/extract_alias_facts.py` / `tools/gen_aliases.py` and
/// the [`crate::world::library`] alias module. Without a path the compiled
/// fallback is validated; with a path that data file is validated (for example
/// an archive override of `data\client\aliases.ron`).
pub fn check_aliases(path: Option<&std::path::Path>) -> Result<crate::world::AliasesSummary, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_alias_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_aliases().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven key binding table (`data\client\keybinds.ron`)
/// and returns the number of bindings.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary exposes
/// it as `ro-next --check-keybinds [PATH]`.
pub fn check_keybinds(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::input::keybinds::validate(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::input::keybinds::validate_compiled().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven paper-doll slot table
/// (`data\client\paper_doll_slots.ron`) and returns the number of equipment
/// slots.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary exposes
/// it as `ro-next --check-paper-doll [PATH]`.
pub fn check_paper_doll(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::interface::components::paper_doll_slots::validate(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::interface::components::paper_doll_slots::validate_compiled().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven skill behaviour and procedural effect data
/// (`archive/data/client/skills.ron`) and returns the row counts.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary
/// exposes it as `ro-next --check-skills [PATH]`.
pub fn check_skills(path: Option<&std::path::Path>) -> Result<crate::world::SkillsSummary, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::world::validate_skill_data(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::world::validate_compiled_skills().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven world/login configuration
/// (`archive/data/client/world_config.ron`) and returns its summary.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary
/// exposes it as `ro-next --check-world [PATH]`.
pub fn check_world_config(path: Option<&std::path::Path>) -> Result<crate::client_config::WorldConfigSummary, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::client_config::validate_world_config(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::client_config::validate_compiled_world_config().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven UI audio paths
/// (`archive/data/client/ui_audio.ron`) and returns their summary.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary
/// exposes it as `ro-next --check-ui-audio [PATH]`.
pub fn check_ui_audio(path: Option<&std::path::Path>) -> Result<crate::client_config::UiAudioSummary, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::client_config::validate_ui_audio(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::client_config::validate_compiled_ui_audio().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates the data-driven debug map warp list
/// (`archive/data/client/debug_maps.ron`) and returns its number of warps.
///
/// `path` selects a concrete data file (for example an archive override);
/// without a path the compiled fallback is validated. The client binary
/// exposes it as `ro-next --check-debug-maps [PATH]`. This is a debug-only
/// convenience list, not user-facing content.
pub fn check_debug_maps(path: Option<&std::path::Path>) -> Result<usize, String> {
    match path {
        Some(path) => {
            let content = std::fs::read_to_string(path).map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            crate::client_config::validate_debug_maps(&content).map_err(|error| format!("{}: {error}", path.display()))
        }
        None => crate::client_config::validate_compiled_debug_maps().map_err(|error| format!("compiled fallback: {error}")),
    }
}

/// Validates every job/body/hair/palette/headgear sprite of the job registry
/// against the real game archives.
pub fn check_appearance() -> Result<crate::world::AppearanceAudit, String> {
    let game_file_loader = GameFileLoader::default();
    game_file_loader.load_archives_from_settings();
    let lua_archive_existed = std::path::Path::new("lua_files.7z").exists();
    game_file_loader.load_patched_lua_files();
    let report = crate::world::jobs::check_appearance(&game_file_loader);
    if !lua_archive_existed {
        game_file_loader.remove_patched_lua_files();
    }
    report
}
