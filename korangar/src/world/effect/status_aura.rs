//! Persistent glow effects for active status effects (buff auras).
//!
//! Several status effects of the official client are drawn as a persistent
//! glow around the affected entity for as long as the status is active. The
//! 2019 client renders the Assumptio visual (`EF_ASSUMPTIO`, effect id 375) in
//! hard-coded C++ code and the reverse-engineered `EffectTable` dump does not
//! contain its data, so this client reproduces it with a procedural glow built
//! from textures that are part of the 2019 data set:
//!
//! * a soft blue-white radial glow and a rotating energy ring on the ground
//!   below the entity (`effect\efst_waterfield2\bubble1.bmp`),
//! * a camera facing soft glow sphere at body height
//!   (`effect\round_sphere.bmp`).
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
//! The framework is a small registry: add a [`StatusAuraSpec`] constant for the
//! status and one arm to [`status_aura_spec`]. The world reconciles the active
//! status effects of [`crate::state::status_effects`] against the spawned auras
//! every frame ([`crate::World`] `update_status_auras`), so nothing else has to
//! be wired up for a new status: start/end packets, local expiry, map change,
//! death, refresh and entities entering/leaving the view are handled centrally.
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
use crate::world::Camera;

/// `EFST_ASSUMPTIO` of the 2019 rAthena status database (Priest skill
/// Assumptio). The status ids are the `EFST_*` ids the map server sends in
/// `ZC_MSG_STATE_CHANGE3`/`ZC_EFST_SET_ENTER`; the generated
/// [`crate::state::status_effect_names`] table maps both ids to "Assumptio".
const EFST_ASSUMPTIO: u16 = 110;
/// `EFST_ASSUMPTIO2` ("Party Assumptio"/cash Assumptio) - the same visual.
const EFST_ASSUMPTIO2: u16 = 473;

/// Height of the body glow of a standing character in world units.
///
/// The head/name label height used elsewhere in the client is 21 world units,
/// so 14 units is roughly two thirds of the character height (upper torso/head
/// area). The glow must be around the body, not around the feet.
const BODY_HEIGHT: f32 = 13.0;
/// Factor applied to the body glow height while the entity is sitting (the
/// sitting head height is 12 of the standing 21 world units).
const SITTING_HEIGHT_FACTOR: f32 = 12.0 / 21.0;
/// Lift of the ground layers above the terrain to avoid z-fighting with the
/// ground depth buffer (the effect pass depth tests against the scene).
const GROUND_LIFT: f32 = 0.15;

/// One persistent glow effect style.
///
/// All sizes are in world units (one map cell is 10 world units) and are
/// multiplied by the entity scale, so a status glow follows a scaled monster
/// as well. The colors are sRGB tints that the textures are multiplied with.
pub struct StatusAuraSpec {
    /// Soft radial glow drawn flat on the ground below the entity.
    pub ground_texture: &'static str,
    /// Rotating ring/energy texture drawn flat on the ground.
    pub ring_texture: &'static str,
    /// Camera facing soft glow drawn at body height.
    pub body_texture: &'static str,
    /// Tint of the ground layers.
    pub ground_color: Color,
    /// Tint of the body glow.
    pub body_color: Color,
    /// Radius of the ground glow in world units.
    pub ground_radius: f32,
    /// Radius of the ground ring in world units.
    pub ring_radius: f32,
    /// Rotation speed of the ground ring in radians per second.
    pub ring_rotation_speed: f32,
    /// Size of the camera facing body glow in world units.
    pub body_size: f32,
    /// Base alpha of the ground glow (before the pulse).
    pub ground_alpha: f32,
    /// Base alpha of the ground ring (before the pulse).
    pub ring_alpha: f32,
    /// Base alpha of the body glow (before the pulse).
    pub body_alpha: f32,
    /// Pulse speed in radians per second; two layers pulse out of phase.
    pub pulse_speed: f32,
    /// Pulse amplitude as a fraction of the base alpha (`0.25` = ±25 %).
    pub pulse_amount: f32,
}

/// Assumptio: a blue-white "shield of light" around the target, matching the
/// official blue/white glow style as closely as the available 2019 textures
/// permit.
const ASSUMPTIO: StatusAuraSpec = StatusAuraSpec {
    ground_texture: "effect\\round_sphere.bmp",
    ring_texture: "effect\\efst_waterfield2\\bubble1.bmp",
    body_texture: "effect\\round_sphere.bmp",
    ground_color: Color::rgb(0.55, 0.85, 1.0),
    body_color: Color::rgb(0.75, 0.92, 1.0),
    ground_radius: 8.0,
    ring_radius: 5.0,
    ring_rotation_speed: 0.8,
    body_size: 9.0,
    ground_alpha: 0.32,
    ring_alpha: 0.55,
    body_alpha: 0.26,
    pulse_speed: 2.2,
    pulse_amount: 0.28,
};

/// Returns the glow style for a status effect index, or `None` if the status
/// has no persistent visual.
///
/// The ids are the `EFST_*` status ids that the map server sends; they are
/// *not* skill ids (skill `HP_ASSUMPTIO` = 361 maps to the one-shot `asum.str`
/// cast effect, which is spawned by the normal skill effect mapping).
pub fn status_aura_spec(status_index: u16) -> Option<&'static StatusAuraSpec> {
    match status_index {
        EFST_ASSUMPTIO | EFST_ASSUMPTIO2 => Some(&ASSUMPTIO),
        _ => None,
    }
}

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
    body_texture: Arc<Texture>,
    entity_id: EntityId,
    center: EffectCenter,
    /// Shared with the world's status-aura bookkeeping: cleared when the status
    /// ends or the entity becomes ineligible.
    active: Arc<AtomicBool>,
    time: f32,
    /// Entity scale of the last update (1.0 for normal characters).
    scale: f32,
    /// Whether the owner is sitting; lowers the body glow.
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
        let body_texture = texture_loader.get_or_load(spec.body_texture, ImageType::Color).ok()?;

        Some(Self {
            spec,
            ground_texture,
            ring_texture,
            body_texture,
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

        // Ground glow and ring, pulsing out of phase so the aura "breathes"
        // instead of flashing uniformly.
        let ground_alpha = self.spec.ground_alpha * pulse(time, self.spec, 0.0);
        let ring_alpha = self.spec.ring_alpha * pulse(time, self.spec, std::f32::consts::PI);
        let ground_color = Color::rgba(
            self.spec.ground_color.red,
            self.spec.ground_color.green,
            self.spec.ground_color.blue,
            ground_alpha,
        );
        let ring_color = Color::rgba(
            self.spec.ground_color.red,
            self.spec.ground_color.green,
            self.spec.ground_color.blue,
            ring_alpha,
        );

        self.render_ground_quad(
            renderer,
            camera,
            anchor,
            &self.ground_texture,
            ground_color,
            self.spec.ground_radius * scale,
            0.0,
        );
        self.render_ground_quad(
            renderer,
            camera,
            anchor,
            &self.ring_texture,
            ring_color,
            self.spec.ring_radius * scale,
            time * self.spec.ring_rotation_speed,
        );

        // Camera facing glow at body height; the glow slowly rotates so the
        // soft texture does not look like a static screen space sprite.
        let body_height = BODY_HEIGHT * scale * if self.sitting { SITTING_HEIGHT_FACTOR } else { 1.0 };
        let body_position = anchor + Vector3::new(0.0, body_height, 0.0);
        let body_alpha = self.spec.body_alpha * pulse(time, self.spec, std::f32::consts::FRAC_PI_2);
        let body_color = Color::rgba(
            self.spec.body_color.red,
            self.spec.body_color.green,
            self.spec.body_color.blue,
            body_alpha,
        );
        let corners = camera_facing_corners(camera, body_position, self.spec.body_size * 0.5 * scale, time * 0.35);
        let texture_coordinates = [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(1.0, 1.0),
            Vector2::new(0.0, 1.0),
        ];

        render_world_quad_blended(
            renderer,
            camera,
            body_position,
            Self::screen_size(renderer),
            self.body_texture.clone(),
            body_color,
            corners,
            texture_coordinates,
            BlendFactor::SrcAlpha,
            BlendFactor::One,
        );
    }

    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.center.to_position())
    }
}

#[cfg(test)]
mod tests {
    use super::{EFST_ASSUMPTIO, EFST_ASSUMPTIO2, StatusAuraSpec, pulse, status_aura_spec};
    use crate::graphics::Color;

    #[test]
    fn assumptio_and_party_assumptio_have_a_glow() {
        assert!(status_aura_spec(EFST_ASSUMPTIO).is_some());
        assert!(status_aura_spec(EFST_ASSUMPTIO2).is_some());
    }

    #[test]
    fn other_statuses_have_no_glow_yet() {
        // Blessing, Increase Agility, Kyrie Eleison, Angelus, Stone Curse,
        // Poison and an unknown id must not spawn an aura.
        for status_index in [0_u16, 9, 10, 12, 28, 875, 883, 30_000] {
            assert!(
                status_aura_spec(status_index).is_none(),
                "status {status_index} has an unexpected glow"
            );
        }
    }

    #[test]
    fn pulse_stays_positive_and_bounded() {
        let spec = StatusAuraSpec {
            ground_texture: "",
            ring_texture: "",
            body_texture: "",
            ground_color: Color::WHITE,
            body_color: Color::WHITE,
            ground_radius: 1.0,
            ring_radius: 1.0,
            ring_rotation_speed: 0.0,
            body_size: 1.0,
            ground_alpha: 0.5,
            ring_alpha: 0.5,
            body_alpha: 0.5,
            pulse_speed: 2.0,
            pulse_amount: 0.25,
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
