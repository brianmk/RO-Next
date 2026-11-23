//! Shared primitives for the hard-coded (procedural) skill effects of the
//! official client.
//!
//! The official Windows client does not play a `.str`/`.spr` file for many
//! skills; it builds the visual from hard-coded primitives. The unified data
//! file `archive/data/client/skill_effects.ron` (see
//! [`crate::world::library::skill_effects`]) maps a skill/phase to one of the
//! primitives in this module via `kind: Primitive` layers:
//!
//! * [`HitFlash`] - a burst of camera-facing quads at the target (melee
//!   impacts, bolt impact flashes, official `EF_HIT`/`2D`/`3D` components),
//! * [`Projectile`] - one or more textured quads travelling from the caster to
//!   the target, or a bolt rain falling onto the target cell,
//! * [`TextureSequence`] - a flipbook billboard for the official client's
//!   multi-texture (`totalTexture`) effect components,
//! * [`SkillAura`] - a cylinder/ring/ground field, reusing the existing
//!   hard-coded aura renderer.
//!
//! Every primitive allocates its state once when it is spawned and only uses
//! fixed-size loops while updating/rendering, so the per-frame cost is bounded
//! and allocation free. Spawning is bounded by the effect budget of
//! [`crate::world::EffectHolder`] (global cap, per-entity cap and distance
//! culling).

use cgmath::{InnerSpace, Point3, Vector3};
use wgpu::BlendFactor;

use crate::graphics::Color;
use crate::world::Camera;
use crate::world::effect::AuraGrow;
use crate::world::library::{LayerGrow, SkillEffectLayer};

mod ground_aura;
mod hit_flash;
mod projectile;
mod texture_sequence;

pub use self::ground_aura::spawn_ground_aura;
pub use self::hit_flash::HitFlash;
pub use self::projectile::{Projectile, ProjectileMode};
pub use self::texture_sequence::TextureSequence;

/// Borrowed parameters of one procedural primitive layer.
///
/// Built on the stack from a [`SkillEffectLayer`] by the skill effect spawner
/// (no allocation, no per-frame work): the primitive implementations only copy
/// the values they need into their own state once when they are spawned.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PrimitiveParams<'a> {
    /// Effect texture relative to `data\texture\`.
    pub texture: &'a str,
    /// Deterministic per-row seed of the burst variation.
    pub seed: u32,
    /// Number of burst quads / projectiles / duplicates.
    pub count: u16,
    /// Lifetime in milliseconds.
    pub duration_ms: u32,
    /// Duplicate delay / projectile stagger in milliseconds.
    pub delay_ms: u32,
    /// Billboard world size at the start.
    pub size: f32,
    /// Billboard world size at the end.
    pub size_end: f32,
    /// Random world offset radius of the burst.
    pub spread: f32,
    /// World height above the anchor (cylinder height for ground auras).
    pub height: f32,
    /// Bottom radius of a ground aura.
    pub bottom_radius: f32,
    /// Top radius of a ground aura.
    pub top_radius: f32,
    /// Layer alpha (`0..=1`).
    pub alpha: f32,
    /// RGB tint of the texture.
    pub color: Color,
    /// Additive blending.
    pub additive: bool,
    /// Ground aura rotation in radians per second.
    pub rotation_speed: f32,
    /// Ground aura growth animation.
    pub grow: AuraGrow,
}

impl<'a> PrimitiveParams<'a> {
    /// Builds the primitive parameters of a `Primitive` layer, or `None` when
    /// the data row has no texture.
    pub(crate) fn from_layer(layer: &'a SkillEffectLayer) -> Option<Self> {
        // A sequence layer has no single `file`; its first frame provides the
        // texture identity for the deterministic seed and the size fields.
        let texture = layer.file.or_else(|| layer.frames.first().copied())?;

        Some(Self {
            texture,
            seed: (texture.len() as u32)
                .wrapping_mul(7919)
                .wrapping_add(layer.count as u32)
                .wrapping_add(layer.duration_ms),
            count: layer.count,
            duration_ms: layer.duration_ms,
            delay_ms: layer.delay_ms,
            size: layer.size,
            size_end: layer.size_end,
            spread: layer.spread,
            height: layer.height,
            bottom_radius: layer.bottom_radius,
            top_radius: layer.top_radius,
            alpha: layer.alpha,
            color: layer.tint,
            additive: layer.additive,
            rotation_speed: layer.rotation_speed,
            grow: match layer.grow {
                LayerGrow::Constant => AuraGrow::Constant,
                LayerGrow::TopRadius => AuraGrow::TopRadius,
                LayerGrow::BothRadii => AuraGrow::BothRadii,
                LayerGrow::Height => AuraGrow::Height,
            },
        })
    }
}

/// Texture coordinates of a full quad, shared by all billboard primitives.
pub(crate) const FULL_QUAD_TEXTURE_COORDINATES: [cgmath::Vector2<f32>; 4] = [
    cgmath::Vector2::new(0.0, 0.0),
    cgmath::Vector2::new(1.0, 0.0),
    cgmath::Vector2::new(1.0, 1.0),
    cgmath::Vector2::new(0.0, 1.0),
];

/// Blend factors of the effect: additive for glows, alpha blending otherwise.
///
/// These are the straight alpha factors of the official client effect data
/// (the same convention as the `.str` frames). The effect textures and colors
/// are premultiplied, so the forward effect pass translates the `SrcAlpha`
/// source factor to `One` before creating the blend state; see
/// `premultiplied_blend_state` in `graphics/passes/forward/effect.rs`.
pub(crate) fn blend_factors(additive: bool) -> (BlendFactor, BlendFactor) {
    match additive {
        true => (BlendFactor::SrcAlpha, BlendFactor::One),
        false => (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
    }
}

/// The four corners of a camera-facing billboard quad.
pub(crate) fn camera_facing_corners(camera: &dyn Camera, position: Point3<f32>, half_size: f32, angle: f32) -> [Point3<f32>; 4] {
    let view_direction = camera.view_direction();
    let right = camera.look_up_vector().cross(view_direction).normalize();
    let up = view_direction.cross(right).normalize();

    let rotated_right = (right * angle.cos() + up * angle.sin()) * half_size;
    let rotated_up = (up * angle.cos() - right * angle.sin()) * half_size;

    [
        position - rotated_right + rotated_up,
        position + rotated_right + rotated_up,
        position + rotated_right - rotated_up,
        position - rotated_right - rotated_up,
    ]
}

/// Deterministic pseudo random number in `[0, 1)`; used for the per-quad
/// variation of the burst primitives (no random number generator state needed).
pub(crate) fn pseudo_random(mut seed: u32) -> f32 {
    seed ^= seed << 13;
    seed ^= seed >> 17;
    seed ^= seed << 5;
    (seed & 0x00FF_FFFF) as f32 / 0x0100_0000 as f32
}

/// Golden angle used to spread the burst quads over a disc.
pub(crate) const GOLDEN_ANGLE: f32 = 2.399_963_2;

/// Convenience for the spawners: the position of an effect center plus an
/// offset (the primitives anchor at the entity/cell feet and raise themselves).
pub(crate) fn raised(center: Point3<f32>, height: f32) -> Point3<f32> {
    center + Vector3::new(0.0, height, 0.0)
}
