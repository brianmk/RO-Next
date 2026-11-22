//! Official skill -> sprite effect (`.spr` + `.act`) mapping.
//!
//! Some official skill visuals are short-lived animated sprites from
//! `data\\sprite\\이팩트\\...`; the Windows client plays them through its
//! hard-coded effect table. This module loads the extracted, GRF-validated
//! mapping:
//!
//! `archive/data/client/skill_sprite_effects.ron`
//!
//! The file is shipped in the client archive and loaded through
//! [`GameFileLoader`], the same loader that reads the GRF archives, so a server
//! can override it (or add effects for a modified data set) without rebuilding
//! the client. The generated Rust fallback embeds the same file with
//! `include_str!`, so the data file and the compiled table can never drift
//! apart.
//!
//! Generation: `tools/gen_skill_sprite_effects.py` from the extracted Windows
//! client tables (`tools/windows_client_sprite_effects.json`,
//! `tools/windows_client_skill_effects.json`) plus the official `CondaRO.exe`
//! sprite whitelist (`tools/windows_client_spr_whitelist.txt`). `--check` is
//! the drift guard. Every referenced `.spr` **and** `.act` was validated
//! against the real GRF archives at generation time.
//!
//! The table is a process-wide static (no per-frame allocations); lookups are a
//! binary search over the sorted `(skill, phase)` keys.

use std::sync::OnceLock;

use cgmath::Vector3;
use korangar_loaders::FileLoader;
use ragnarok_packets::SkillId;
use serde::Deserialize;

use super::skill_effect_assets::SkillEffectVariant;
use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\skill_sprite_effects.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/skill_sprite_effects.ron");

/// Where a sprite effect is anchored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum SpriteEffectAnchor {
    /// On the caster entity; follows it while it is alive.
    Caster,
    /// At the target of the skill (falls back to the caster for self casts).
    Target,
    /// At the ground cell of the skill (falls back to the caster when the
    /// packet does not carry a position).
    Ground,
}

/// One row of the generated data file.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SpriteEffectSpec {
    pub skill: u16,
    pub variant: SkillEffectVariant,
    pub anchor: SpriteEffectAnchor,
    /// Candidate sprite/action stems relative to `data\sprite\`, without the
    /// `.spr`/`.act` extension (for example
    /// `이팩트\\su_lunaticcarrotbeat\\su_lunaticcarrotbeat`). The first pair
    /// that exists in the archives is played.
    pub files: Vec<String>,
    /// World-space offset from the anchor's feet.
    pub offset: (f32, f32, f32),
    /// Pixel scale of the sprite (1.0 = authored size).
    pub scale: f32,
    /// Playback duration in milliseconds; `0` uses the native ACT frame timing.
    pub duration_ms: u32,
}

impl SpriteEffectSpec {
    pub(crate) fn offset(&self) -> Vector3<f32> {
        Vector3::new(self.offset.0, self.offset.1, self.offset.2)
    }
}

#[derive(Deserialize)]
struct SpriteEffectFile {
    #[allow(dead_code)]
    provenance: String,
    effects: Vec<SpriteEffectSpec>,
}

static EFFECTS: OnceLock<Vec<SpriteEffectSpec>> = OnceLock::new();

fn parse(content: &str) -> Result<Vec<SpriteEffectSpec>, String> {
    let file: SpriteEffectFile = ron::from_str(content).map_err(|error| error.to_string())?;
    let mut effects = file.effects;
    effects.sort_by_key(|effect| (effect.skill, effect.variant.phase()));
    Ok(effects)
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

    let effects = match parse(content) {
        Ok(effects) => effects,
        Err(error) => {
            crate::logging::log_warn!("skill sprite effects: invalid {DATA_PATH} ({error}), using the compiled fallback");
            match parse(FALLBACK) {
                Ok(effects) => effects,
                Err(fallback_error) => {
                    crate::logging::log_warn!("skill sprite effects: compiled fallback is invalid too ({fallback_error})");
                    Vec::new()
                }
            }
        }
    };

    let count = effects.len();
    let _ = EFFECTS.set(effects);
    crate::logging::log_debug!("skill sprite effects: loaded {count} entries from the {source}");
    count
}

/// The sprite effects of `skill_id` for one spawn phase.
///
/// Returns an empty slice when the data file has no entry for the skill/phase;
/// callers then keep the `.str`/procedural layers and the generic cast circle.
pub(crate) fn skill_sprite_effects(skill_id: SkillId, variant: SkillEffectVariant) -> &'static [SpriteEffectSpec] {
    let Some(effects) = EFFECTS.get() else {
        return &[];
    };

    let key = (skill_id.0, variant.phase());
    let start = effects.partition_point(|effect| (effect.skill, effect.variant.phase()) < key);
    let end = effects.partition_point(|effect| (effect.skill, effect.variant.phase()) <= key);

    &effects[start..end]
}

/// Whether the mapping has any sprite effect for `skill_id`.
#[allow(dead_code)]
pub(crate) fn skill_has_sprite_effect(skill_id: SkillId) -> bool {
    EFFECTS
        .get()
        .is_some_and(|effects| effects.iter().any(|effect| effect.skill == skill_id.0))
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::{FALLBACK, SkillEffectVariant, SpriteEffectAnchor, SpriteEffectSpec, parse, skill_sprite_effects};

    fn load_fallback() {
        if super::EFFECTS.get().is_none() {
            let effects = parse(FALLBACK).expect("the compiled sprite effect fallback has to be valid");
            let _ = super::EFFECTS.set(effects);
        }
    }

    fn rows(skill_id: u16, variant: SkillEffectVariant) -> &'static [SpriteEffectSpec] {
        skill_sprite_effects(SkillId(skill_id), variant)
    }

    #[test]
    fn data_file_is_valid_and_has_entries() {
        load_fallback();

        // AL_BLESSING: official `축복` sprite, success phase, raised to the head.
        let blessing = rows(34, SkillEffectVariant::Success);
        assert_eq!(blessing.len(), 1);
        assert_eq!(blessing[0].files, vec!["이팩트\\축복".to_owned()]);
        assert_eq!(blessing[0].anchor, SpriteEffectAnchor::Caster);
        assert_eq!(blessing[0].duration_ms, 1500);
        assert!(blessing[0].offset().y > 20.0);

        // New World `su_*` skill: name-matched official sprite.
        let carrot = rows(5036, SkillEffectVariant::Success);
        assert_eq!(carrot.len(), 1);
        assert_eq!(carrot[0].files, vec![
            "이팩트\\su_lunaticcarrotbeat\\su_lunaticcarrotbeat".to_owned()
        ]);
        assert_eq!(carrot[0].anchor, SpriteEffectAnchor::Target);

        // Catnip Meteor is a ground sprite.
        assert_eq!(rows(5028, SkillEffectVariant::Ground)[0].anchor, SpriteEffectAnchor::Ground);

        // Spirit Marble spawns the three coloured marbles around the caster.
        assert_eq!(rows(5024, SkillEffectVariant::Success).len(), 3);

        // Classic hit effect from the official effect table.
        let tracking = rows(512, SkillEffectVariant::Hit);
        assert_eq!(tracking.len(), 1);
        assert_eq!(tracking[0].files, vec!["이팩트\\트래킹".to_owned()]);
        assert_eq!(tracking[0].anchor, SpriteEffectAnchor::Target);

        // An unmapped skill has no entries and never panics.
        assert!(rows(1, SkillEffectVariant::Success).is_empty());
    }

    #[test]
    fn every_row_has_a_sprite_pair_and_a_positive_scale() {
        load_fallback();

        let effects = super::EFFECTS.get().expect("fallback loaded");
        assert!(!effects.is_empty());
        for effect in effects {
            assert!(!effect.files.is_empty(), "skill {} has no candidate sprite", effect.skill);
            assert!(effect.files.iter().all(|file| !file.is_empty()));
            assert!(effect.scale > 0.0, "skill {} has an invalid scale", effect.skill);
            assert!(
                effect.duration_ms <= 30_000,
                "skill {} has an unreasonable duration",
                effect.skill
            );
        }
    }

    #[test]
    fn phase_lookup_separates_skills_and_variants() {
        load_fallback();

        // AL_HOLYWATER is a success sprite only.
        assert!(!rows(31, SkillEffectVariant::Success).is_empty());
        assert!(rows(31, SkillEffectVariant::Hit).is_empty());
        // Fire Wall is not a sprite-effect skill.
        assert!(rows(18, SkillEffectVariant::Ground).is_empty());
    }

    #[test]
    fn malformed_data_is_rejected_without_panicking() {
        assert!(parse("not ron").is_err());
        assert!(
            parse(
                "(provenance: \"x\", effects: [(skill: 1, variant: Nope, anchor: Caster, files: [], offset: (0.0, 0.0, 0.0), scale: 1.0, \
                 duration_ms: 0)])"
            )
            .is_err()
        );
    }
}
