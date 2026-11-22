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
use crate::world::effect::procedural::PrimitiveParams;
use crate::world::effect::{EffectCenter, SkillAura, SkillAuraSpec};

/// Spawns a ground aura/cylinder for one data-file row. Returns `None` when the
/// effect texture is not part of the game data.
pub fn spawn_ground_aura(texture_loader: &TextureLoader, center: EffectCenter, spec: &PrimitiveParams<'_>) -> Option<SkillAura> {
    let texture: Arc<Texture> = texture_loader.get_or_load(spec.texture, ImageType::Color).ok()?;

    let aura_spec = SkillAuraSpec {
        duration_ms: spec.duration_ms.max(100),
        height: spec.height.max(0.0),
        bottom_radius: spec.bottom_radius.max(0.0),
        top_radius: spec.top_radius.max(0.0),
        alpha_max: spec.alpha.clamp(0.0, 1.0),
        additive: spec.additive,
        rotation_speed: spec.rotation_speed,
        grow: spec.grow,
        duplicates: spec.count.clamp(1, 16) as usize,
        duplicate_delay_ms: spec.delay_ms,
        color: Color::rgba(spec.color.red, spec.color.green, spec.color.blue, 1.0),
    };

    Some(SkillAura::with_texture(texture, center, aura_spec))
}
