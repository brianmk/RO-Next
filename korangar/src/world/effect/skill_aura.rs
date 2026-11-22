//! Hard-coded skill aura effects (the "cylinder" family of the original
//! client).
//!
//! The original client does not use `.str` files for several of its hard-coded
//! skill effects. Those effects are *procedural* cylinders built from an effect
//! texture (`data\texture\effect\ring_yellow.tga`, `alpha_center.tga`, ...):
//!
//! * `EF_BEGINSPELL` (12, effect 12 of the client effect table): the yellow
//!   cone that appears around the caster when a skill starts. Its top radius
//!   grows from zero while its bottom radius stays small.
//! * `EF_GUMGANG2` (261, "Fury Cast Animation"): four expanding yellow rings
//!   around the caster, spawned 100 ms apart.
//! * `EF_PORTAL5` (402) is *not* a cylinder, it is the caster body flash that
//!   is implemented by the entity flash color support.
//!
//! The geometry, timing and colors mirror the official client's hard-coded
//! effect data (as reconstructed by the roBrowser legacy client
//! `DB/Effects/EffectTable.js`). Sizes in the client effect table are expressed
//! in units of 1/5 map cell; the world uses 10 world units per cell, so one
//! official effect unit is [`OFFICIAL_EFFECT_UNIT`] world units.
//!
//! The effect is fully procedural: it allocates nothing per frame and renders a
//! fixed number of screen-space quads per frame (one per cylinder segment and
//! duplicate).

use std::sync::Arc;

use cgmath::{Point3, Vector2, Vector3};
use wgpu::BlendFactor;

use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::{EffectBase, EffectCenter, render_world_quad_blended};

/// World units per official effect unit.
///
/// The official effect data uses 5 units per map cell (the same convention as
/// the client's hard-coded effect primitives) and this client uses 10 world
/// units per map cell.
pub const OFFICIAL_EFFECT_UNIT: f32 = 2.0;

/// Number of cylinder segments. The official client uses 20 sides for its
/// effect cylinders.
const CYLINDER_SEGMENTS: usize = 20;

/// How the radii of a cylinder grow while the effect is alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuraGrow {
    /// Only the top radius grows (the cast cone of `EF_BEGINSPELL`).
    TopRadius,
    /// Both radii grow with the elapsed time (`EF_GUMGANG2`'s expanding rings).
    BothRadii,
    /// The radii stay constant for the whole lifetime (data-driven ground
    /// rings).
    Constant,
    /// The radii stay constant while the *height* grows from the ground (the
    /// green `EF_HEAL` restoring column of the official client).
    Height,
}

/// Description of one client "cylinder" effect. All values are tuned to the
/// official client's hard-coded effect table.
#[derive(Debug, Clone, Copy)]
pub struct SkillAuraSpec {
    /// Effect texture relative to `data\texture\` (for example
    /// `effect\ring_yellow.tga`).
    pub texture_path: &'static str,
    /// Duration of a single cylinder in milliseconds.
    pub duration_ms: u32,
    /// Height of the cylinder in world units.
    pub height: f32,
    /// Bottom radius in world units.
    pub bottom_radius: f32,
    /// Top radius in world units.
    pub top_radius: f32,
    /// Maximum alpha of the effect.
    pub alpha_max: f32,
    /// Additive blending (`SrcAlpha`/`One`) instead of alpha blending.
    pub additive: bool,
    /// Rotation of the cylinder around its vertical axis in radians/second.
    pub rotation_speed: f32,
    /// Radius growth over the lifetime.
    pub grow: AuraGrow,
    /// Number of duplicate cylinders.
    pub duplicates: usize,
    /// Delay between duplicates in milliseconds.
    pub duplicate_delay_ms: u32,
    /// Base tint of the effect (the texture is multiplied with it).
    pub color: Color,
}

impl SkillAuraSpec {
    /// Total lifetime including the delayed duplicates in milliseconds.
    pub const fn total_duration_ms(&self) -> u32 {
        self.duration_ms + self.duplicate_delay_ms * (self.duplicates as u32 - 1)
    }

    /// Straight alpha factors of the official cylinder effect (translated to
    /// premultiplied alpha by the forward effect pass).
    fn blend_factors(&self) -> (BlendFactor, BlendFactor) {
        match self.additive {
            true => (BlendFactor::SrcAlpha, BlendFactor::One),
            false => (BlendFactor::SrcAlpha, BlendFactor::OneMinusSrcAlpha),
        }
    }
}

/// A live skill aura attached to an entity (or a world position).
pub struct SkillAura {
    texture: Arc<Texture>,
    center: EffectCenter,
    spec: SkillAuraSpec,
    time: f32,
    gets_deleted: bool,
}

impl SkillAura {
    /// Returns `None` if the game files do not contain the effect texture, in
    /// which case the skill is simply not accompanied by the aura.
    pub fn new(texture_loader: &TextureLoader, center: EffectCenter, spec: SkillAuraSpec) -> Option<Self> {
        let texture = texture_loader.get_or_load(spec.texture_path, ImageType::Color).ok()?;

        Some(Self {
            texture,
            center,
            spec,
            time: 0.0,
            gets_deleted: false,
        })
    }

    /// Creates an aura from an already loaded texture (used by the data-driven
    /// procedural effect spec, whose textures are resolved at runtime).
    pub fn with_texture(texture: Arc<Texture>, center: EffectCenter, spec: SkillAuraSpec) -> Self {
        Self {
            texture,
            center,
            spec,
            time: 0.0,
            gets_deleted: false,
        }
    }

    /// Alpha of a single cylinder after `elapsed` milliseconds. The original
    /// client fades the effect in over the first quarter and out over the last
    /// quarter of its lifetime.
    fn cylinder_alpha(spec: &SkillAuraSpec, elapsed: f32) -> f32 {
        let duration = spec.duration_ms as f32;
        let fade_in = duration * 0.25;
        let fade_out_start = duration * 0.75;

        let fade = if elapsed < fade_in {
            elapsed / fade_in
        } else if elapsed > fade_out_start {
            (duration - elapsed) / (duration - fade_out_start)
        } else {
            1.0
        };

        (fade.clamp(0.0, 1.0)) * spec.alpha_max
    }

    /// Bottom and top radius of a single cylinder after `elapsed` milliseconds,
    /// following the official effect's growth animation.
    fn cylinder_radii(spec: &SkillAuraSpec, elapsed: f32) -> (f32, f32) {
        let progress = (elapsed / spec.duration_ms as f32).clamp(0.0, 1.0);

        match spec.grow {
            AuraGrow::TopRadius => (spec.bottom_radius, spec.top_radius * progress),
            AuraGrow::BothRadii => (spec.bottom_radius * progress, spec.top_radius * progress),
            AuraGrow::Constant => (spec.bottom_radius, spec.top_radius),
            // Constant radii: only the height grows (heal column).
            AuraGrow::Height => (spec.bottom_radius, spec.top_radius),
        }
    }

    /// Height of a cylinder after `elapsed` milliseconds. `AuraGrow::Height`
    /// effects grow from the ground over their first second, like the official
    /// client's cylinder animation 1.
    fn cylinder_height(spec: &SkillAuraSpec, elapsed: f32) -> f32 {
        match spec.grow {
            AuraGrow::Height => spec.height * (elapsed / 1_000.0).clamp(0.0, 1.0),
            _ => spec.height,
        }
    }

    fn render_cylinder(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, anchor: Point3<f32>, elapsed_ms: f32) {
        let (bottom_radius, top_radius) = Self::cylinder_radii(&self.spec, elapsed_ms);
        let alpha = Self::cylinder_alpha(&self.spec, elapsed_ms);
        let height = Self::cylinder_height(&self.spec, elapsed_ms);

        if alpha <= 0.001 || (bottom_radius <= 0.05 && top_radius <= 0.05) {
            return;
        }

        let screen_size = ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        };
        let (source_blend_factor, destination_blend_factor) = self.spec.blend_factors();
        let color = Color::rgba(self.spec.color.red, self.spec.color.green, self.spec.color.blue, alpha);
        let angle = elapsed_ms * 0.001 * self.spec.rotation_speed;

        // A zero-height cylinder is a flat ground ring/decal: draw one horizontal
        // textured quad instead of a degenerate cylinder.
        if self.spec.height <= 0.01 {
            let radius = bottom_radius.max(top_radius);
            // Ground decals are drawn a hair above the terrain to avoid
            // z-fighting with the ground depth buffer.
            let anchor = anchor + Vector3::new(0.0, 0.15, 0.0);
            let (sin, cos) = angle.sin_cos();
            let right = Vector3::new(cos * radius, 0.0, sin * radius);
            let forward = Vector3::new(-sin * radius, 0.0, cos * radius);
            let corners = [
                anchor - right + forward,
                anchor + right + forward,
                anchor + right - forward,
                anchor - right - forward,
            ];
            let texture_coordinates = [
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
                Vector2::new(1.0, 1.0),
                Vector2::new(0.0, 1.0),
            ];

            render_world_quad_blended(
                renderer,
                camera,
                anchor,
                screen_size,
                self.texture.clone(),
                color,
                corners,
                texture_coordinates,
                source_blend_factor,
                destination_blend_factor,
            );
            return;
        }

        let segment_angle = std::f32::consts::TAU / CYLINDER_SEGMENTS as f32;
        let height = Vector3::new(0.0, height, 0.0);

        for segment in 0..CYLINDER_SEGMENTS {
            let angle_start = angle + segment as f32 * segment_angle;
            let angle_end = angle + (segment + 1) as f32 * segment_angle;

            let bottom_start = anchor + Vector3::new(angle_start.cos() * bottom_radius, 0.0, angle_start.sin() * bottom_radius);
            let bottom_end = anchor + Vector3::new(angle_end.cos() * bottom_radius, 0.0, angle_end.sin() * bottom_radius);
            let top_start = anchor + Vector3::new(angle_start.cos() * top_radius, 0.0, angle_start.sin() * top_radius) + height;
            let top_end = anchor + Vector3::new(angle_end.cos() * top_radius, 0.0, angle_end.sin() * top_radius) + height;

            let texture_start = segment as f32 / CYLINDER_SEGMENTS as f32;
            let texture_end = (segment + 1) as f32 / CYLINDER_SEGMENTS as f32;

            render_world_quad_blended(
                renderer,
                camera,
                anchor,
                screen_size,
                self.texture.clone(),
                color,
                [top_start, top_end, bottom_start, bottom_end],
                [
                    Vector2::new(texture_start, 0.0),
                    Vector2::new(texture_end, 0.0),
                    Vector2::new(texture_start, 1.0),
                    Vector2::new(texture_end, 1.0),
                ],
                source_blend_factor,
                destination_blend_factor,
            );
        }
    }
}

impl EffectBase for SkillAura {
    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.center.to_position())
    }

    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.center.follow_entity(entities);
        self.time += delta_time;

        !self.gets_deleted && (self.time * 1000.0) < self.spec.total_duration_ms() as f32
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut crate::world::PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        let anchor = self.center.to_position();
        let elapsed_total_ms = self.time * 1000.0;

        for duplicate in 0..self.spec.duplicates {
            let elapsed = elapsed_total_ms - duplicate as f32 * self.spec.duplicate_delay_ms as f32;

            if elapsed < 0.0 || elapsed > self.spec.duration_ms as f32 {
                continue;
            }

            self.render_cylinder(renderer, camera, anchor, elapsed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AuraGrow, OFFICIAL_EFFECT_UNIT, SkillAura, SkillAuraSpec};
    use crate::graphics::Color;

    fn begin_spell() -> SkillAuraSpec {
        SkillAuraSpec {
            texture_path: "effect\\ring_yellow.tga",
            duration_ms: 1_000,
            height: 4.0 * OFFICIAL_EFFECT_UNIT,
            bottom_radius: 1.0 * OFFICIAL_EFFECT_UNIT,
            top_radius: 5.0 * OFFICIAL_EFFECT_UNIT,
            color: Color::rgb(0.95, 0.85, 0.35),
            alpha_max: 0.8,
            additive: true,
            rotation_speed: 0.0,
            grow: AuraGrow::TopRadius,
            duplicates: 1,
            duplicate_delay_ms: 0,
        }
    }

    fn gumgang2() -> SkillAuraSpec {
        SkillAuraSpec {
            texture_path: "effect\\ring_yellow.tga",
            duration_ms: 1_500,
            height: 2.0 * OFFICIAL_EFFECT_UNIT,
            bottom_radius: 2.0 * OFFICIAL_EFFECT_UNIT,
            top_radius: 5.0 * OFFICIAL_EFFECT_UNIT,
            color: Color::rgb(0.95, 0.85, 0.35),
            alpha_max: 0.5,
            additive: false,
            rotation_speed: 4.0,
            grow: AuraGrow::BothRadii,
            duplicates: 4,
            duplicate_delay_ms: 100,
        }
    }

    #[test]
    fn begin_spell_grows_the_top_radius_from_zero() {
        let spec = begin_spell();
        let (bottom_start, top_start) = SkillAura::cylinder_radii(&spec, 0.0);
        let (bottom_mid, top_mid) = SkillAura::cylinder_radii(&spec, 500.0);
        let (bottom_end, top_end) = SkillAura::cylinder_radii(&spec, 1_000.0);

        assert_eq!(bottom_start, spec.bottom_radius);
        assert_eq!(bottom_mid, spec.bottom_radius);
        assert_eq!(bottom_end, spec.bottom_radius);
        assert_eq!(top_start, 0.0);
        assert!(top_mid > 0.0 && top_mid < spec.top_radius);
        assert_eq!(top_end, spec.top_radius);
    }

    #[test]
    fn gumgang2_rings_expand_both_radii_and_last_until_the_delayed_duplicates_end() {
        let spec = gumgang2();
        let (bottom_start, top_start) = SkillAura::cylinder_radii(&spec, 0.0);
        let (bottom_end, top_end) = SkillAura::cylinder_radii(&spec, 1_500.0);

        assert_eq!(bottom_start, 0.0);
        assert_eq!(top_start, 0.0);
        assert_eq!(bottom_end, spec.bottom_radius);
        assert_eq!(top_end, spec.top_radius);
        // Four duplicates 100 ms apart, each living 1500 ms.
        assert_eq!(spec.total_duration_ms(), 1_800);
    }

    fn heal_column() -> SkillAuraSpec {
        SkillAuraSpec {
            texture_path: "effect\\ring_white.tga",
            duration_ms: 1_500,
            height: 8.0 * OFFICIAL_EFFECT_UNIT,
            bottom_radius: 0.95 * OFFICIAL_EFFECT_UNIT,
            top_radius: 0.95 * OFFICIAL_EFFECT_UNIT,
            color: Color::rgb(0.7, 1.0, 0.7),
            alpha_max: 0.2,
            additive: true,
            rotation_speed: 4.36,
            grow: AuraGrow::Height,
            duplicates: 1,
            duplicate_delay_ms: 0,
        }
    }

    #[test]
    fn heal_column_grows_from_the_ground_and_keeps_its_radius() {
        let spec = heal_column();

        assert_eq!(SkillAura::cylinder_height(&spec, 0.0), 0.0);
        assert!((SkillAura::cylinder_height(&spec, 500.0) - spec.height * 0.5).abs() < 1e-5);
        assert_eq!(SkillAura::cylinder_height(&spec, 1_000.0), spec.height);
        assert_eq!(SkillAura::cylinder_height(&spec, 1_500.0), spec.height);

        let (bottom, top) = SkillAura::cylinder_radii(&spec, 750.0);
        assert_eq!(bottom, spec.bottom_radius);
        assert_eq!(top, spec.top_radius);
    }

    #[test]
    fn alpha_fades_in_and_out_and_stays_within_the_limits() {
        let spec = begin_spell();
        let start = SkillAura::cylinder_alpha(&spec, 0.0);
        let peak = SkillAura::cylinder_alpha(&spec, 500.0);
        let end = SkillAura::cylinder_alpha(&spec, 1_000.0);

        assert_eq!(start, 0.0);
        assert!((peak - spec.alpha_max).abs() < 1e-5);
        assert_eq!(end, 0.0);
    }
}
