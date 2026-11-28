//! Persistent glow effects for active status effects (buff auras).
//!
//! Several status effects are drawn as a persistent glow around the affected
//! entity for as long as the status is active. The visuals are **data**: each
//! glow style is a `PersistentGlow` aura spec in the client data file
//! `archive/data/client/status_effects.ron` (generated from the official
//! server/client data plus the curated overlay
//! `tools/data/status_effect_custom.ron` by `tools/gen_status_effects.py`).
//! Adding or tuning an aura is a data row, not a Rust change;
//! [`status_aura_spec`] is only a thin lookup into the status-effect data
//! table.
//!
//! The first data row is Assumptio (`EFST_ASSUMPTIO` 110 / `EFST_ASSUMPTIO2`
//! 473): the official 2019 client renders `EF_ASSUMPTIO` in hard-coded C++ and
//! the reverse-engineered effect table does not contain its data, so the client
//! reproduces it with a procedural shield of light built from textures of the
//! 2019 data set:
//!
//! * a soft blue-white light pool and a slowly rotating energy ring on the
//!   ground below the entity (`effect\whitelight.tga` and
//!   `effect\efst_waterfield2\bubble1.bmp`),
//! * a soft camera facing light plus the animated blue energy shield of the
//!   2019 water field status aura
//!   (`effect\efst_waterfield2\aurawaterfield1..4.tga`) around the body, so the
//!   glow covers the character instead of hiding in the ground below it,
//! * the two camera facing body layers are moved a little towards the camera so
//!   they render *over* the character sprite instead of being half occluded by
//!   its depth.
//!
//! The effect is fully procedural: it allocates once per (entity, status) pair
//! when the status starts, follows the entity every frame and only uses
//! fixed-size loops while rendering, so the per-frame cost is bounded and
//! allocation free. It is attached with [`EffectHolder::add_unit`], which means
//! the global/per-entity effect caps, distance culling and the effect cleanup
//! on map change / entity removal all apply unchanged.
//!
//! # Adding another status glow
//!
//! Add a data row to `tools/data/status_effect_custom.ron` (a `PersistentGlow`
//! spec for the status ids) and regenerate the data file; nothing in this
//! module has to change. The world reconciles the active status effects of
//! [`crate::state::status_effects`] against the spawned auras every frame
//! ([`crate::World`] `update_status_auras`), so start/end packets, local
//! expiry, map change, death, refresh and entities entering/leaving the view
//! are handled centrally.
//!
//! [`EffectHolder::add_unit`]: crate::world::EffectHolder

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use cgmath::{Point3, Vector2, Vector3};
use ragnarok_packets::EntityId;
use wgpu::BlendFactor;

use super::procedural::{FULL_QUAD_TEXTURE_COORDINATES, camera_facing_corners};
use super::{EffectBase, EffectCenter, render_world_quad_blended};
use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::{Camera, StatusAuraKind, StatusAuraSpec};

/// Height factor applied to the body layers while the entity is sitting (the
/// sitting head height is 12 of the standing 21 world units). This is a
/// character-geometry constant shared by every glow style, not aura data.
const SITTING_HEIGHT_FACTOR: f32 = 12.0 / 21.0;
/// Lift of the ground layers above the terrain to avoid z-fighting with the
/// ground depth buffer (the effect pass depth tests against the scene).
///
/// The scene uses reverse-z, so a lifted decal gets a slightly *larger* NDC
/// depth and passes the `GreaterEqual` effect depth test. The value is a
/// noticeable fraction of a world unit instead of a few centimetres because the
/// depth buffer precision at the default camera distance is coarse enough for a
/// 0.15 unit lift to z-fight on sloped cobblestone. Shared by every style.
const GROUND_LIFT: f32 = 1.0;

/// Multiplier for a pulsing layer at the given time.
fn pulse(time: f32, spec: &StatusAuraSpec, phase: f32) -> f32 {
    (1.0 + spec.pulse_amount * (time * spec.pulse_speed + phase).sin()).max(0.0)
}

/// A persistent glow owned by one entity, driven by one of its active status
/// effects.
pub struct StatusAura {
    spec: &'static StatusAuraSpec,
    ground_texture: Arc<Texture>,
    ring_texture: Arc<Texture>,
    body_frames: Vec<Arc<Texture>>,
    body_glow_texture: Arc<Texture>,
    entity_id: EntityId,
    center: EffectCenter,
    /// Shared with the world's status-aura bookkeeping: cleared when the status
    /// ends or the entity becomes ineligible.
    active: Arc<AtomicBool>,
    time: f32,
    /// Entity scale of the last update (1.0 for normal characters).
    scale: f32,
    /// Whether the owner is sitting; lowers the body layers.
    sitting: bool,
    gets_deleted: bool,
}

impl StatusAura {
    /// Loads the textures of `spec` and creates the glow around an entity.
    ///
    /// Returns `None` if one of the status glow textures is not part of the
    /// game data; the status then simply stays without a glow.
    pub fn new(
        texture_loader: &TextureLoader,
        entity_id: EntityId,
        spec: &'static StatusAuraSpec,
        active: Arc<AtomicBool>,
    ) -> Option<Self> {
        let ground_texture = texture_loader.get_or_load(spec.ground_texture, ImageType::Color).ok()?;
        let ring_texture = texture_loader.get_or_load(spec.ring_texture, ImageType::Color).ok()?;

        let mut body_frames = Vec::with_capacity(spec.body_frames.len());
        for frame in spec.body_frames {
            body_frames.push(texture_loader.get_or_load(frame, ImageType::Color).ok()?);
        }

        if body_frames.is_empty() {
            return None;
        }

        let body_glow_texture = texture_loader.get_or_load(spec.body_glow_texture, ImageType::Color).ok()?;

        Some(Self {
            spec,
            ground_texture,
            ring_texture,
            body_frames,
            body_glow_texture,
            entity_id,
            center: EffectCenter::Entity(entity_id, Point3::new(0.0, 0.0, 0.0)),
            active,
            time: 0.0,
            scale: 1.0,
            sitting: false,
            gets_deleted: false,
        })
    }

    /// Screen size of the current window, needed by the quad renderer.
    fn screen_size(renderer: &EffectRenderer) -> ScreenSize {
        ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        }
    }

    /// Renders one horizontal textured quad on the ground.
    #[allow(clippy::too_many_arguments)]
    fn render_ground_quad(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        anchor: Point3<f32>,
        texture: &Arc<Texture>,
        color: Color,
        radius: f32,
        angle: f32,
    ) {
        if color.alpha <= 0.001 || radius <= 0.01 {
            return;
        }

        let anchor = anchor + Vector3::new(0.0, GROUND_LIFT, 0.0);
        let (sin, cos) = angle.sin_cos();
        let right = Vector3::new(cos * radius, 0.0, sin * radius);
        let forward = Vector3::new(-sin * radius, 0.0, cos * radius);
        let corners = [
            anchor - right + forward,
            anchor + right + forward,
            anchor + right - forward,
            anchor - right - forward,
        ];

        render_world_quad_blended(
            renderer,
            camera,
            anchor,
            Self::screen_size(renderer),
            texture.clone(),
            color,
            corners,
            FULL_QUAD_TEXTURE_COORDINATES,
            BlendFactor::SrcAlpha,
            BlendFactor::One,
        );
    }

    /// Renders one camera facing textured quad at `position`.
    #[allow(clippy::too_many_arguments)]
    fn render_body_quad(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        position: Point3<f32>,
        size: f32,
        angle: f32,
        texture: &Arc<Texture>,
        color: Color,
    ) {
        if color.alpha <= 0.001 || size <= 0.01 {
            return;
        }

        let corners = camera_facing_corners(camera, position, size * 0.5, angle);
        let texture_coordinates = [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 1.0),
        ];

        render_world_quad_blended(
            renderer,
            camera,
            position,
            Self::screen_size(renderer),
            texture.clone(),
            color,
            corners,
            texture_coordinates,
            BlendFactor::SrcAlpha,
            BlendFactor::One,
        );
    }

    /// Current frame of the animated body shield.
    fn body_frame(&self) -> &Arc<Texture> {
        let frame_count = self.body_frames.len().max(1);
        let frame = (self.time * self.spec.body_frame_rate).max(0.0) as usize % frame_count;

        &self.body_frames[frame]
    }
}

impl EffectBase for StatusAura {
    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        if self.gets_deleted || !self.active.load(Ordering::Relaxed) {
            return false;
        }

        self.center.follow_entity(entities);

        // The owner left the view, died or is fading out: the glow must go away
        // with it. The world reconciles the bookkeeping entry in the same frame,
        // so a later 0x0984 (status list on re-entering the view) respawns it.
        let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == self.entity_id) else {
            return false;
        };

        if entity.is_dead() || entity.is_fading() {
            return false;
        }

        self.sitting = entity.is_sitting();
        self.scale = entity.get_scale().clamp(0.25, 4.0);
        self.time += delta_time;

        true
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
        let scale = self.scale;
        let time = self.time;
        let spec = self.spec;

        match spec.kind {
            StatusAuraKind::PersistentGlow => self.render_persistent_glow(renderer, camera, anchor, scale, time),
        }
    }

    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.center.to_position())
    }
}

impl StatusAura {
    /// Renders the `PersistentGlow` style: ground light pool + rotating ring +
    /// camera facing body light and animated body shield.
    fn render_persistent_glow(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, anchor: Point3<f32>, scale: f32, time: f32) {
        let spec = self.spec;

        // Ground light pool and rotating energy ring, pulsing out of phase so
        // the aura "breathes" instead of flashing uniformly.
        let ground_alpha = spec.ground_alpha * pulse(time, spec, 0.0);
        let ring_alpha = spec.ring_alpha * pulse(time, spec, std::f32::consts::PI);
        let ground_color = Color::rgba(
            spec.ground_color.red,
            spec.ground_color.green,
            spec.ground_color.blue,
            ground_alpha,
        );
        let ring_color = Color::rgba(
            spec.ground_color.red,
            spec.ground_color.green,
            spec.ground_color.blue,
            ring_alpha,
        );

        self.render_ground_quad(
            renderer,
            camera,
            anchor,
            &self.ground_texture,
            ground_color,
            spec.ground_radius * scale,
            0.0,
        );
        self.render_ground_quad(
            renderer,
            camera,
            anchor,
            &self.ring_texture,
            ring_color,
            spec.ring_radius * scale,
            time * spec.ring_rotation_speed,
        );

        // Camera facing body layers. They are lifted to the middle of the upper
        // body and moved a little towards the camera so the character sprite
        // cannot depth-occlude them: the shield has to be visible *over* the
        // character, which is what the player expects from the buff.
        let body_height = spec.body_height * scale * if self.sitting { SITTING_HEIGHT_FACTOR } else { 1.0 };
        let towards_camera = -camera.view_direction() * spec.body_camera_offset * scale;
        let body_position = anchor + Vector3::new(0.0, body_height, 0.0) + towards_camera;

        let body_glow_alpha = spec.body_glow_alpha * pulse(time, spec, std::f32::consts::FRAC_PI_2);
        let glow_color = Color::rgba(
            spec.glow_color.red,
            spec.glow_color.green,
            spec.glow_color.blue,
            body_glow_alpha,
        );

        self.render_body_quad(
            renderer,
            camera,
            body_position,
            spec.body_glow_size * scale,
            time * 0.25,
            &self.body_glow_texture,
            glow_color,
        );

        let body_alpha = spec.body_alpha * pulse(time, spec, std::f32::consts::PI * 0.75);
        let body_color = Color::rgba(spec.body_color.red, spec.body_color.green, spec.body_color.blue, body_alpha);

        self.render_body_quad(
            renderer,
            camera,
            body_position,
            spec.body_size * scale,
            time * spec.body_rotation_speed,
            self.body_frame(),
            body_color,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::pulse;
    use crate::graphics::Color;
    use crate::world::{StatusAuraKind, StatusAuraSpec, status_aura_spec};

    #[test]
    fn assumptio_has_no_procedural_aura() {
        // Assumptio is `render.flags & 8`, the white pulse outline ported in
        // `world/entity/common.rs`; it must not spawn a separate status glow.
        for status_index in [110_u16, 473] {
            assert!(
                status_aura_spec(status_index).is_none(),
                "status {status_index} must not have a procedural aura"
            );
        }
    }

    #[test]
    fn only_documented_statuses_have_a_persistent_aura() {
        // Blessing (10), Increase Agility (12), Angelus (9) and Improve
        // Concentration (3) must not leave a persistent glow.
        for status_index in [3_u16, 9, 10, 12] {
            assert!(
                status_aura_spec(status_index).is_none(),
                "status {status_index} must not have a persistent aura"
            );
        }

        // Statuses without a data row (and unknown ids) never spawn a glow.
        for status_index in [0_u16, 28, 110, 473, 875, 883, 30_000] {
            assert!(
                status_aura_spec(status_index).is_none(),
                "status {status_index} has an unexpected glow"
            );
        }
    }

    #[test]
    fn every_aura_spec_is_renderable() {
        // The renderer only supports `PersistentGlow`; the data loader rejects
        // other kinds, so every spec reachable here must be valid.
        for status_index in [2_u16, 25, 31, 68, 86, 87, 107, 123, 161, 165, 166, 167, 188, 232, 235, 437] {
            let spec = status_aura_spec(status_index).unwrap_or_else(|| panic!("status {status_index} must have an aura"));
            assert_eq!(spec.kind, StatusAuraKind::PersistentGlow);
            assert!((0.0..=1.0).contains(&spec.ground_alpha));
            assert!((0.0..=1.0).contains(&spec.ring_alpha));
            assert!((0.0..=1.0).contains(&spec.body_alpha));
            assert!((0.0..=1.0).contains(&spec.body_glow_alpha));
            assert!((0.0..=1.0).contains(&spec.pulse_amount));
            assert!(spec.ground_radius > 0.0 && spec.ring_radius > 0.0);
            assert!(spec.body_size > 0.0 && spec.body_glow_size > 0.0);
        }
    }

    #[test]
    fn steel_body_aura_is_scaled_like_the_official_fan() {
        // `EFST_STEELBODY` (87) draws the official `SuperSlashFan(4, 0)` fan
        // (`RagEffect.cpp:24181-24183`): prim 35 `RenderOrbitIcons`
        // (`EffectPrimitiveRendering.cpp:4764-4897`) puts 4.95-unit icon quads
        // (`distance * scale = 3.5` half-diagonal) on a 4-unit orbit, so the
        // whole fan reaches `4 + 3.5 = 7.5` world units. The raw effect-table
        // 24-unit body quad (half-size 12) read oversized in-game, so Steel
        // Body keeps the same 0.7 scale as the accepted Fury aura
        // (commit 1dc4f547) instead of the un-scaled effect-table radii.
        let steel = status_aura_spec(87).expect("Steel Body must have a persistent aura");

        assert_eq!(steel.kind, StatusAuraKind::PersistentGlow);
        assert_eq!(steel.body_frames, &[
            "effect\\super1.bmp",
            "effect\\super2.bmp",
            "effect\\super3.bmp",
            "effect\\super4.bmp",
            "effect\\super5.bmp",
        ]);
        assert_eq!(steel.body_glow_texture, "effect\\super1.bmp");
        assert_eq!(
            (steel.ground_radius, steel.ring_radius, steel.body_size, steel.body_glow_size),
            (7.35, 4.9, 16.8, 9.8)
        );
        assert_eq!(steel.body_height, 8.4);

        // Fury (86) shows the same super1..5 fan, so both auras must use the
        // same scale; this locks the data row against drifting back to the
        // raw 24-unit effect-table quad.
        let fury = status_aura_spec(86).expect("Fury must have a persistent aura");
        assert_eq!(
            (
                steel.ground_radius,
                steel.ring_radius,
                steel.body_size,
                steel.body_glow_size,
                steel.body_height
            ),
            (
                fury.ground_radius,
                fury.ring_radius,
                fury.body_size,
                fury.body_glow_size,
                fury.body_height
            )
        );

        // The body shield's outer edge now sits next to the official 7.5-unit
        // fan reach (the raw data put it at 12.0).
        assert!(
            (steel.body_size * 0.5 - 7.5).abs() < 1.5,
            "steel body aura edge {}",
            steel.body_size * 0.5
        );
    }

    #[test]
    fn pulse_stays_positive_and_bounded() {
        let spec = StatusAuraSpec {
            kind: StatusAuraKind::PersistentGlow,
            ground_texture: "",
            ring_texture: "",
            body_frames: &[""],
            body_glow_texture: "",
            ground_color: Color::WHITE,
            body_color: Color::WHITE,
            glow_color: Color::WHITE,
            ground_radius: 1.0,
            ring_radius: 1.0,
            ring_rotation_speed: 0.0,
            body_size: 1.0,
            body_glow_size: 1.0,
            ground_alpha: 0.5,
            ring_alpha: 0.5,
            body_alpha: 0.5,
            body_glow_alpha: 0.5,
            body_frame_rate: 1.0,
            body_rotation_speed: 0.0,
            body_camera_offset: 0.0,
            pulse_speed: 2.0,
            pulse_amount: 0.25,
            body_height: 12.0,
        };

        // Sample a full pulse period; the factor must never go negative (an
        // inverted factor would produce a black glow) and never exceed the
        // configured amplitude.
        for step in 0..1_000 {
            let time = step as f32 * 0.01;
            let factor = pulse(time, &spec, 0.0);
            assert!(factor >= 0.0);
            assert!(factor <= 1.0 + spec.pulse_amount);
        }
    }
}
