//! Data-driven ground aura / cylinder primitive.
//!
//! The official client draws its ground skill effects (`CYLINDER` components:
//! Sanctuary, Magnus, Fire Pillar, ground property fields, level-99 style
//! auras) as cylinders/rings around a cell. [`SkillAura`] already implements
//! that renderer for the hard-coded cast cone/rings; this module builds an aura
//! from a data-file row instead of a compile-time constant.

use std::sync::Arc;

use crate::graphics::{Color, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::world::effect::{AuraGrow, EffectCenter, SkillAura, SkillAuraSpec};
use crate::world::library::ProceduralEffectSpec;

/// Spawns a ground aura/cylinder for one data-file row. Returns `None` when the
/// effect texture is not part of the game data.
pub fn spawn_ground_aura(texture_loader: &TextureLoader, center: EffectCenter, spec: &ProceduralEffectSpec) -> Option<SkillAura> {
    let texture: Arc<Texture> = texture_loader.get_or_load(&spec.texture, ImageType::Color).ok()?;

    let aura_spec = SkillAuraSpec {
        // The texture of a data-driven aura is already loaded above; the path is
        // only used by `SkillAura::new` for the compile-time aura table.
        texture_path: "",
        duration_ms: spec.duration_ms.max(100),
        height: spec.height.max(0.0),
        bottom_radius: spec.bottom_radius.max(0.0),
        top_radius: spec.top_radius.max(0.0),
        alpha_max: spec.alpha.clamp(0.0, 1.0),
        additive: spec.additive,
        rotation_speed: spec.rotation_speed,
        grow: AuraGrow::Constant,
        duplicates: spec.count.clamp(1, 8) as usize,
        duplicate_delay_ms: spec.delay_ms,
        color: Color::rgba(spec.color.0, spec.color.1, spec.color.2, 1.0),
    };

    Some(SkillAura::with_texture(texture, center, aura_spec))
}
