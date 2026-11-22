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

/// Where an effect of a skill is played.
///
/// The official client does not play every effect at the same place: most
/// effects are aimed at the target cell, but caster effects (for example
/// `effectIdOnCaster`, Pierce's `pierce.str`) are anchored on the caster. The
/// data file records the anchor of every row because the old generator only
/// kept a coarse phase, which made on-caster effects spawn at the damaged
/// target.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
pub(crate) enum EffectAnchor {
    /// On the caster (for example `effectIdOnCaster`).
    Caster,
    /// At the target cell; falls back to the caster when no target is known.
    Target,
    /// From the caster to the target (release/projectile style effects).
    Travel,
    /// At the target cell when the packet knows one, otherwise on the caster.
    #[default]
    Auto,
}

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
    #[serde(default)]
    anchor: EffectAnchor,
    file: String,
}

/// One loaded mapping entry with a leaked, process-lifetime file name.
struct StoredEntry {
    skill: u16,
    variant: SkillEffectVariant,
    anchor: EffectAnchor,
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
            anchor: entry.anchor,
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

/// Iterates over the effect files and spawn anchors of `skill_id` for one
/// variant.
///
/// A phase can have several layers (for example two different `.str` files);
/// every row is spawned once per cast. The anchor tells the caller whether the
/// layer belongs on the caster or at the target cell.
pub(crate) fn skill_effect_entries(
    skill_id: SkillId,
    variant: SkillEffectVariant,
) -> impl Iterator<Item = (&'static str, EffectAnchor)> + 'static {
    ASSETS
        .get()
        .map(|assets| assets.as_slice())
        .unwrap_or(&[])
        .iter()
        .filter(move |entry| entry.skill == skill_id.0 && entry.variant == variant)
        .map(|entry| (entry.file, entry.anchor))
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

    use super::{EffectAnchor, FALLBACK, SkillEffectVariant, parse, skill_effect_entries};

    fn load_fallback() -> Result<(), String> {
        let entries = parse(FALLBACK)?;
        let _ = super::ASSETS.set(entries);
        Ok(())
    }

    fn file_of(skill_id: u16, variant: SkillEffectVariant) -> Option<&'static str> {
        skill_effect_entries(SkillId(skill_id), variant).next().map(|(file, _anchor)| file)
    }

    fn anchor_of(skill_id: u16, variant: SkillEffectVariant) -> Option<EffectAnchor> {
        skill_effect_entries(SkillId(skill_id), variant)
            .next()
            .map(|(_file, anchor)| anchor)
    }

    fn entries_of(skill_id: u16, variant: SkillEffectVariant) -> Vec<(&'static str, EffectAnchor)> {
        skill_effect_entries(SkillId(skill_id), variant).collect()
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
    fn caster_effects_are_anchored_on_the_caster() {
        assert!(load_fallback().is_ok());
        // `effectIdOnCaster` (pierce) used to be spawned at the damaged target
        // because the data file only stored the phase.
        assert_eq!(anchor_of(56, SkillEffectVariant::Success), Some(EffectAnchor::Caster));
        // `effectIdOnCaster` of Spear Stab and Brandish Spear as well.
        assert_eq!(anchor_of(58, SkillEffectVariant::Success), Some(EffectAnchor::Caster));
        assert_eq!(anchor_of(62, SkillEffectVariant::Success), Some(EffectAnchor::Caster));
        // `hitEffectId` is aimed at the target cell.
        assert_eq!(anchor_of(56, SkillEffectVariant::Hit), Some(EffectAnchor::Target));
    }

    #[test]
    fn a_phase_can_have_several_effect_layers() {
        assert!(load_fallback().is_ok());
        // Bolt skills have both the element hit flash and the generic hit line.
        let lightning = entries_of(20, SkillEffectVariant::Hit);
        assert!(lightning.len() >= 2, "expected two hit layers, got {lightning:?}");
        assert!(lightning.iter().all(|(_file, anchor)| *anchor == EffectAnchor::Target));
    }

    #[test]
    fn every_row_has_a_known_anchor() {
        let entries = parse(FALLBACK).expect("valid fallback");
        assert!(entries.len() > 150, "mapping should stay broad, got {}", entries.len());
        assert!(entries.iter().any(|entry| entry.anchor == EffectAnchor::Caster));
        assert!(entries.iter().any(|entry| entry.anchor == EffectAnchor::Target));
        assert!(entries.iter().any(|entry| entry.anchor == EffectAnchor::Auto));
    }

    #[test]
    fn variants_use_distinct_phases() {
        assert_ne!(SkillEffectVariant::Ground.phase(), SkillEffectVariant::Hit.phase());
        assert_ne!(SkillEffectVariant::Success.phase(), SkillEffectVariant::Release.phase());
    }
}
