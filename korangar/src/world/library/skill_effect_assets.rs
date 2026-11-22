//! Official skill -> effect asset mapping.
//!
//! The mapping is a **data file** shipped in the client archive
//! (`archive/data/client/skill_effect_assets.ron`) and is loaded at runtime
//! through [`GameFileLoader`], the same loader that reads the GRF archives. A
//! server can therefore override the mapping (or add effects for a modified
//! data set) by shipping the file in its own `data.grf`/`CondaRO.grf`/`data\`
//! folder, without rebuilding the client.
//!
//! Generation: `tools/gen_skill_effect_assets.py`. The generator extracts the
//! numeric skill/effect ids from the Windows client tables
//! (`tools/windows_client_skill_effects.json`), resolves the effect file names,
//! validates every file against the real GRF archives and the `CondaRO.exe`
//! effect whitelist (`tools/windows_client_effect_whitelist.txt`), and writes
//! the RON file. `--check` is the drift guard.
//!
//! The same RON file is embedded via `include_str!` as the fallback when the
//! data file is missing, so the compiled fallback and the data file can never
//! drift apart. Effects are looked up with a binary search; the loaded table is
//! a process-wide static (no per-frame allocations).

use std::sync::OnceLock;

use korangar_loaders::FileLoader;
use ragnarok_packets::SkillId;
use serde::Deserialize;

use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\skill_effect_assets.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/skill_effect_assets.ron");

/// When an effect of a skill is played.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub(crate) enum SkillEffectVariant {
    /// Ground-target skill completion, at the target cell.
    Ground,
    /// Cast start, on the caster.
    Cast,
    /// Successful no-damage/cast completion, on the caster.
    Success,
    /// Targeted skill lands, at the target.
    Release,
    /// Skill damage hits, at the target.
    Hit,
    /// Skill damage packet arrives, on the caster.
    Caster,
}

impl SkillEffectVariant {
    /// Stable phase id used by the completion-effect de-duplicator.
    pub(crate) fn phase(self) -> u8 {
        self as u8
    }
}

#[derive(Deserialize)]
struct SkillEffectAssetFile {
    #[allow(dead_code)]
    provenance: String,
    effects: Vec<SkillEffectEntry>,
}

#[derive(Deserialize)]
struct SkillEffectEntry {
    skill: u16,
    variant: SkillEffectVariant,
    file: String,
}

/// One loaded mapping entry with a leaked, process-lifetime file name.
struct StoredEntry {
    skill: u16,
    variant: SkillEffectVariant,
    file: &'static str,
}

static ASSETS: OnceLock<Vec<StoredEntry>> = OnceLock::new();

fn parse(content: &str) -> Result<Vec<StoredEntry>, String> {
    let file: SkillEffectAssetFile = ron::from_str(content).map_err(|error| error.to_string())?;
    let mut entries: Vec<StoredEntry> = file
        .effects
        .into_iter()
        .map(|entry| StoredEntry {
            skill: entry.skill,
            variant: entry.variant,
            file: Box::leak(entry.file.into_boxed_str()),
        })
        .collect();
    entries.sort_by_key(|entry| (entry.skill, entry.variant, entry.file));
    Ok(entries)
}

/// Loads the mapping from the client archive (or the compiled fallback).
///
/// Returns the number of loaded entries. A malformed data file is reported and
/// the compiled fallback is used instead, so a bad override can never leave the
/// client without a usable mapping.
pub(crate) fn load(game_file_loader: &GameFileLoader) -> usize {
    let from_archive = game_file_loader
        .get(DATA_PATH)
        .ok()
        .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());

    let (content, source) = match from_archive.as_deref() {
        Some(content) => (content, "archive"),
        None => (FALLBACK, "compiled fallback"),
    };

    let entries = match parse(content) {
        Ok(entries) => entries,
        Err(error) => {
            crate::logging::log_warn!("skill effect assets: invalid {DATA_PATH} ({error}), using the compiled fallback");
            match parse(FALLBACK) {
                Ok(entries) => entries,
                Err(fallback_error) => {
                    crate::logging::log_warn!("skill effect assets: compiled fallback is invalid too ({fallback_error})");
                    Vec::new()
                }
            }
        }
    };

    let count = entries.len();
    let _ = ASSETS.set(entries);
    crate::logging::log_debug!("skill effect assets: loaded {count} entries from the {source}");
    count
}

/// Iterates over the effect files of `skill_id` for one variant.
pub(crate) fn skill_effect_files(skill_id: SkillId, variant: SkillEffectVariant) -> impl Iterator<Item = &'static str> + 'static {
    ASSETS
        .get()
        .map(|assets| assets.as_slice())
        .unwrap_or(&[])
        .iter()
        .filter(move |entry| entry.skill == skill_id.0 && entry.variant == variant)
        .map(|entry| entry.file)
}

/// Whether the mapping has any entry for `skill_id`.
pub(crate) fn skill_has_effect(skill_id: SkillId) -> bool {
    ASSETS
        .get()
        .map(|assets| assets.iter().any(|entry| entry.skill == skill_id.0))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use korangar_loaders::FileLoader;
    use ragnarok_packets::SkillId;

    use super::{FALLBACK, SkillEffectVariant, parse, skill_effect_files};

    fn load_fallback() -> Result<(), String> {
        let entries = parse(FALLBACK)?;
        let _ = super::ASSETS.set(entries);
        Ok(())
    }

    fn file_of(skill_id: u16, variant: SkillEffectVariant) -> Option<&'static str> {
        skill_effect_files(SkillId(skill_id), variant).next()
    }

    #[test]
    fn data_file_is_valid_and_has_entries() {
        assert!(load_fallback().is_ok());
        // Storm Gust comes from the official ground/success table.
        assert_eq!(file_of(89, SkillEffectVariant::Success), Some("stormgust.str"));
        // Thunderstorm is a ground effect entry.
        assert_eq!(file_of(21, SkillEffectVariant::Ground), Some("thunderstorm.str"));
        // Pierce has both a completion and a hit variant.
        assert_eq!(file_of(56, SkillEffectVariant::Success), Some("pierce.str"));
        assert!(file_of(56, SkillEffectVariant::Hit).is_some());
        // Fire Wall uses the official template picked by the generator.
        assert_eq!(file_of(18, SkillEffectVariant::Ground), Some("firewall1.str"));
        // Improve Concentration.
        assert_eq!(file_of(45, SkillEffectVariant::Success), Some("concentration.str"));
        // An unmapped skill has no entries.
        assert_eq!(file_of(1, SkillEffectVariant::Success), None);
    }

    #[test]
    fn variants_use_distinct_phases() {
        assert_ne!(SkillEffectVariant::Ground.phase(), SkillEffectVariant::Hit.phase());
        assert_ne!(SkillEffectVariant::Success.phase(), SkillEffectVariant::Release.phase());
    }
}
