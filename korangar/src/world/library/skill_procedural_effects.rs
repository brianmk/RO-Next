//! Procedural (hard-coded) skill effect spec.
//!
//! The official Windows client does not use a data file for many skill visuals;
//! it builds them from hard-coded primitives. This module loads the generated
//! mapping from skill/phase to one of the client's shared primitives:
//!
//! `archive/data/client/skill_procedural_effects.ron`
//!
//! The file is shipped in the client archive and loaded through
//! [`GameFileLoader`], the same loader that reads the GRF archives, so a server
//! can override it (or tune the effect values) without rebuilding the client.
//! The generated Rust fallback embeds the same file with `include_str!`, so the
//! data file and the compiled table can never drift apart.
//!
//! Generation: `tools/gen_skill_procedural_effects.py` from the extracted facts
//! in `tools/windows_client_procedural_effects.json`; the audit table
//! `tools/skill_procedural_components.csv` documents which skills need a
//! procedural component. `--check` is the drift guard.
//!
//! The table is a process-wide static (no per-frame allocations); lookups are a
//! binary search over the sorted `(skill, phase)` keys.

use std::sync::OnceLock;

use korangar_loaders::FileLoader;
use ragnarok_packets::SkillId;
use serde::Deserialize;

use super::skill_effect_assets::SkillEffectVariant;
use crate::loaders::GameFileLoader;

/// Data file path inside the client archive (`data\client\...`).
const DATA_PATH: &str = "data\\client\\skill_procedural_effects.ron";
/// Fallback copy of the same file, compiled into the client.
const FALLBACK: &str = include_str!("../../../archive/data/client/skill_procedural_effects.ron");

/// Where a procedural effect spawns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ProceduralAnchor {
    /// On the caster entity (follows it while it is alive).
    Caster,
    /// At the target cell/entity of the skill.
    Target,
    /// Flies from the caster to the target.
    Travel,
    /// Falls from above onto the target cell.
    Rain,
    /// The target when the packet reports one, the caster for self casts.
    Auto,
}

/// The shared primitive that renders the effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) enum ProceduralPrimitive {
    /// A burst of camera-facing billboard quads.
    HitFlash,
    /// One or more quads travelling/raining towards the target.
    Projectile,
    /// A cylinder / ring / ground field.
    GroundAura,
}

/// One row of the generated data file.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ProceduralEffectSpec {
    pub skill: u16,
    pub variant: SkillEffectVariant,
    pub anchor: ProceduralAnchor,
    pub primitive: ProceduralPrimitive,
    /// Effect texture relative to `data\texture\` (for example
    /// `effect\pok3.tga`), validated against the GRFs by the generator.
    pub texture: String,
    pub count: u16,
    pub duration_ms: u32,
    pub delay_ms: u32,
    /// World size of the billboard/projectile quads.
    pub size: f32,
    pub size_end: f32,
    /// Random world offset radius of the quad burst.
    pub spread: f32,
    /// Height offset above the anchor's feet in world units.
    pub height: f32,
    /// Bottom/top radius of a cylinder aura in world units.
    pub bottom_radius: f32,
    pub top_radius: f32,
    pub alpha: f32,
    /// RGB tint of the texture.
    pub color: (f32, f32, f32),
    pub additive: bool,
    pub rotation_speed: f32,
}

#[derive(Deserialize)]
struct ProceduralEffectFile {
    #[allow(dead_code)]
    provenance: String,
    effects: Vec<ProceduralEffectSpec>,
}

static EFFECTS: OnceLock<Vec<ProceduralEffectSpec>> = OnceLock::new();

fn parse(content: &str) -> Result<Vec<ProceduralEffectSpec>, String> {
    let file: ProceduralEffectFile = ron::from_str(content).map_err(|error| error.to_string())?;
    let mut effects = file.effects;
    effects.sort_by_key(|effect| (effect.skill, effect.variant.phase()));
    Ok(effects)
}

/// Loads the mapping from the client archive (or the compiled fallback).
///
/// Returns the number of loaded entries. A malformed data file is reported and
/// the compiled fallback is used instead, so a bad override can never leave the
/// client without a usable spec.
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
            crate::logging::log_warn!("skill procedural effects: invalid {DATA_PATH} ({error}), using the compiled fallback");
            match parse(FALLBACK) {
                Ok(effects) => effects,
                Err(fallback_error) => {
                    crate::logging::log_warn!("skill procedural effects: compiled fallback is invalid too ({fallback_error})");
                    Vec::new()
                }
            }
        }
    };

    let count = effects.len();
    let _ = EFFECTS.set(effects);
    crate::logging::log_debug!("skill procedural effects: loaded {count} entries from the {source}");
    count
}

/// The procedural effects of `skill_id` for one spawn phase.
///
/// Returns an empty slice when the data file has no entry for the skill/phase;
/// callers then fall back to the `.str` mapping and the generic cast circle.
pub(crate) fn skill_procedural_effects(skill_id: SkillId, variant: SkillEffectVariant) -> &'static [ProceduralEffectSpec] {
    let Some(effects) = EFFECTS.get() else {
        return &[];
    };

    let key = (skill_id.0, variant.phase());
    let start = effects.partition_point(|effect| (effect.skill, effect.variant.phase()) < key);
    let end = effects.partition_point(|effect| (effect.skill, effect.variant.phase()) <= key);

    &effects[start..end]
}

/// Whether the spec has any procedural effect for `skill_id`.
pub(crate) fn skill_has_procedural_effect(skill_id: SkillId) -> bool {
    EFFECTS
        .get()
        .is_some_and(|effects| effects.iter().any(|effect| effect.skill == skill_id.0))
}

#[cfg(test)]
mod tests {
    use ragnarok_packets::SkillId;

    use super::{
        FALLBACK, ProceduralAnchor, ProceduralEffectSpec, ProceduralPrimitive, SkillEffectVariant, parse, skill_procedural_effects,
    };

    fn load_fallback() {
        if super::EFFECTS.get().is_none() {
            let effects = parse(FALLBACK).expect("the compiled procedural effect fallback has to be valid");
            let _ = super::EFFECTS.set(effects);
        }
    }

    fn rows(skill_id: u16, variant: SkillEffectVariant) -> &'static [ProceduralEffectSpec] {
        skill_procedural_effects(SkillId(skill_id), variant)
    }

    #[test]
    fn data_file_is_valid_and_covers_the_audited_skills() {
        load_fallback();
        let effects = super::EFFECTS.get().expect("loaded");

        assert!(
            effects.len() >= 240,
            "expected the full procedural spec, got {} rows",
            effects.len()
        );

        let covered_skills = effects.iter().map(|effect| effect.skill).collect::<std::collections::HashSet<_>>();
        // The audit classifies 121 procedural-only + 22 data+procedural skills.
        assert!(
            covered_skills.len() >= 139,
            "expected at least 139 covered skills, got {}",
            covered_skills.len()
        );
    }

    #[test]
    fn melee_hits_have_a_hit_flash_at_the_target() {
        load_fallback();
        let hit_rows = rows(5, SkillEffectVariant::Hit);
        assert!(!hit_rows.is_empty(), "Bash has to have a hit flash");
        assert!(
            hit_rows
                .iter()
                .all(|effect| effect.anchor == ProceduralAnchor::Target && effect.primitive == ProceduralPrimitive::HitFlash)
        );
    }

    #[test]
    fn bolts_rain_onto_the_target() {
        load_fallback();
        let hit_rows = rows(14, SkillEffectVariant::Hit);
        assert!(!hit_rows.is_empty(), "Cold Bolt has to have a procedural hit effect");
        assert!(
            hit_rows
                .iter()
                .any(|effect| { effect.primitive == ProceduralPrimitive::Projectile && effect.anchor == ProceduralAnchor::Rain })
        );
    }

    #[test]
    fn fireballs_travel_from_the_caster() {
        load_fallback();
        let caster_rows = rows(17, SkillEffectVariant::Caster);
        assert!(!caster_rows.is_empty(), "Fire Ball has to throw projectiles");
        assert!(
            caster_rows
                .iter()
                .any(|effect| { effect.primitive == ProceduralPrimitive::Projectile && effect.anchor == ProceduralAnchor::Travel })
        );
    }

    #[test]
    fn ground_skills_have_a_ground_aura() {
        load_fallback();
        let ground_rows = rows(70, SkillEffectVariant::Ground);
        assert!(!ground_rows.is_empty(), "Sanctuary has to have a ground aura");
        assert!(ground_rows.iter().all(|effect| effect.primitive == ProceduralPrimitive::GroundAura));
    }

    #[test]
    fn unknown_skills_have_no_procedural_effects() {
        load_fallback();
        assert!(rows(1, SkillEffectVariant::Hit).is_empty());
        assert!(!super::skill_has_procedural_effect(SkillId(1)));
    }

    #[test]
    fn every_texture_is_an_effect_texture_and_parameters_are_sane() {
        load_fallback();
        let effects = super::EFFECTS.get().expect("loaded");

        for effect in effects {
            assert!(
                effect.texture.starts_with("effect\\"),
                "skill {}: texture '{}' is not relative to data\\texture\\",
                effect.skill,
                effect.texture
            );
            assert!(!effect.texture.is_empty());
            assert!(effect.count >= 1 && effect.count <= 10);
            assert!(effect.duration_ms >= 50 && effect.duration_ms <= 6000);
            assert!((0.0..=1.0).contains(&effect.alpha));
            assert!(effect.size >= 0.0 && effect.size <= 120.0);
            assert!(effect.bottom_radius <= 120.0 && effect.top_radius <= 120.0);
            assert!(effect.color.0.is_finite() && effect.color.1.is_finite() && effect.color.2.is_finite());
        }
    }

    #[test]
    fn all_phases_are_covered() {
        load_fallback();
        let effects = super::EFFECTS.get().expect("loaded");
        let variants = effects
            .iter()
            .map(|effect| effect.variant)
            .collect::<std::collections::HashSet<_>>();

        for variant in [
            SkillEffectVariant::Ground,
            SkillEffectVariant::Cast,
            SkillEffectVariant::Success,
            SkillEffectVariant::Release,
            SkillEffectVariant::Hit,
            SkillEffectVariant::Caster,
        ] {
            assert!(variants.contains(&variant), "no procedural effects for phase {variant:?}");
        }
    }

    #[test]
    fn phases_use_distinct_lookup_keys() {
        load_fallback();
        assert_ne!(SkillEffectVariant::Ground.phase(), SkillEffectVariant::Hit.phase());
        // Fire Ball has rows for two different phases; both lookups must be
        // non-empty and point at different table ranges.
        let caster_rows = rows(17, SkillEffectVariant::Caster);
        let hit_rows = rows(17, SkillEffectVariant::Hit);
        assert!(!caster_rows.is_empty() && !hit_rows.is_empty());
        assert_ne!(caster_rows.as_ptr(), hit_rows.as_ptr());
    }
}
