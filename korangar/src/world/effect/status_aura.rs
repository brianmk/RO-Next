//! Persistent glow effects for active status effects (buff auras).
//!
//! Several status effects of the official client are drawn as a persistent
//! glow around the affected entity for as long as the status is active. The
//! 2019 client renders the Assumptio visual (`EF_ASSUMPTIO`, effect id 375) in
//! hard-coded C++ code and the reverse-engineered `EffectTable` dump does not
//! contain its data, so this client reproduces it with a procedural shield of
//! light built from textures that are part of the 2019 data set:
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

/// Height of the body layers of a standing character in world units.
///
/// The head/name label height used elsewhere in the client is 21 world units,
/// so 13 units is the middle of the upper body. The shield around that point is
/// larger than the character, so the glow covers the body instead of only the
/// feet.
const BODY_HEIGHT: f32 = 13.0;
/// Factor applied to the body layer height while the entity is sitting (the
/// sitting head height is 12 of the standing 21 world units).
const SITTING_HEIGHT_FACTOR: f32 = 12.0 / 21.0;
/// Lift of the ground layers above the terrain to avoid z-fighting with the
/// ground depth buffer (the effect pass depth tests against the scene).
///
/// The scene uses reverse-z, so a lifted decal gets a slightly *larger* NDC
/// depth and passes the `GreaterEqual` effect depth test. The value is a
/// noticeable fraction of a world unit instead of a few centimetres because the
/// depth buffer precision at the default camera distance is coarse enough for a
/// 0.15 unit lift to z-fight on sloped cobblestone.
const GROUND_LIFT: f32 = 1.0;

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
    /// Camera facing animated shield frames, played in order (looping).
    pub body_frames: &'static [&'static str],
    /// Soft camera facing light drawn around the body.
    pub body_glow_texture: &'static str,
    /// Tint of the ground layers.
    pub ground_color: Color,
    /// Tint of the animated body shield.
    pub body_color: Color,
    /// Tint of the soft camera facing body light.
    pub glow_color: Color,
    /// Radius of the ground glow in world units.
    pub ground_radius: f32,
    /// Radius of the ground ring in world units.
    pub ring_radius: f32,
    /// Rotation speed of the ground ring in radians per second.
    pub ring_rotation_speed: f32,
    /// Size (diameter) of the camera facing body shield in world units.
    pub body_size: f32,
    /// Size (diameter) of the camera facing body light in world units.
    pub body_glow_size: f32,
    /// Base alpha of the ground glow (before the pulse).
    pub ground_alpha: f32,
    /// Base alpha of the ground ring (before the pulse).
    pub ring_alpha: f32,
    /// Base alpha of the body shield (before the pulse).
    pub body_alpha: f32,
    /// Base alpha of the body light (before the pulse).
    pub body_glow_alpha: f32,
    /// Playback rate of the animated body shield in frames per second.
    pub body_frame_rate: f32,
    /// Rotation speed of the body shield in radians per second.
    pub body_rotation_speed: f32,
    /// Distance (world units) the camera facing body layers are moved towards
    /// the camera, so they are drawn in front of the entity sprite instead of
    /// being occluded by its depth.
    pub body_camera_offset: f32,
    /// Pulse speed in radians per second; the layers pulse out of phase.
    pub pulse_speed: f32,
    /// Pulse amplitude as a fraction of the base alpha (`0.25` = ±25 %).
    pub pulse_amount: f32,
}

/// Assumptio: a blue-white "shield of light" around the target, matching the
/// official blue/white glow style as closely as the available 2019 textures
/// permit.
///
/// The sizes are deliberately larger than the character (21 world units tall):
/// the animated `aurawaterfield` shield is 26 units across and the ground light
/// pool 22 units across, so the aura is clearly visible over and around the
/// entity at the default camera distance instead of being a small speck at its
/// feet.
const ASSUMPTIO: StatusAuraSpec = StatusAuraSpec {
    ground_texture: "effect\\whitelight.tga",
    ring_texture: "effect\\efst_waterfield2\\bubble1.bmp",
    body_frames: &[
        "effect\\efst_waterfield2\\aurawaterfield1.tga",
        "effect\\efst_waterfield2\\aurawaterfield2.tga",
        "effect\\efst_waterfield2\\aurawaterfield3.tga",
        "effect\\efst_waterfield2\\aurawaterfield4.tga",
    ],
    body_glow_texture: "effect\\whitelight.tga",
    ground_color: Color::rgb(0.45, 0.8, 1.0),
    body_color: Color::rgb(0.65, 0.88, 1.0),
    glow_color: Color::rgb(0.8, 0.95, 1.0),
    ground_radius: 11.0,
    ring_radius: 7.5,
    ring_rotation_speed: 0.8,
    body_size: 26.0,
    body_glow_size: 15.0,
    ground_alpha: 0.42,
    ring_alpha: 0.65,
    body_alpha: 0.6,
    body_glow_alpha: 0.38,
    body_frame_rate: 8.0,
    body_rotation_speed: 0.35,
    body_camera_offset: 2.5,
    pulse_speed: 2.2,
    pulse_amount: 0.22,
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
        let body_height = BODY_HEIGHT * scale * if self.sitting { SITTING_HEIGHT_FACTOR } else { 1.0 };
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

    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.center.to_position())
    }
}

#[cfg(test)]
mod tests {
    use super::{ASSUMPTIO, EFST_ASSUMPTIO, EFST_ASSUMPTIO2, StatusAuraSpec, pulse, status_aura_spec};
    use crate::graphics::Color;

    #[test]
    fn assumptio_and_party_assumptio_have_a_glow() {
        assert!(status_aura_spec(EFST_ASSUMPTIO).is_some());
        assert!(status_aura_spec(EFST_ASSUMPTIO2).is_some());
        // Party/cash Assumptio shares the same visual as the normal buff.
        assert!(std::ptr::eq(
            status_aura_spec(EFST_ASSUMPTIO).unwrap(),
            status_aura_spec(EFST_ASSUMPTIO2).unwrap()
        ));
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
    fn assumptio_glow_is_large_enough_to_cover_the_character() {
        // A character is 21 world units tall. The body shield must be larger
        // than the character (so the glow is visible over the whole body) and
        // the ground light pool must be at least as wide as the character is
        // tall, otherwise the aura is only a speck at the feet (the reason the
        // user reported that Assumptio shows no glow at all).
        let spec = &ASSUMPTIO;

        assert!(spec.body_size >= 24.0, "body shield too small: {}", spec.body_size);
        assert!(spec.body_glow_size >= 12.0, "body light too small: {}", spec.body_glow_size);
        assert!(
            spec.ground_radius * 2.0 >= 21.0,
            "ground light too small: {}",
            spec.ground_radius
        );
        assert!(spec.body_alpha >= 0.5, "body shield too faint: {}", spec.body_alpha);
        assert!(spec.ground_alpha >= 0.35, "ground light too faint: {}", spec.ground_alpha);

        // Every layer must use a texture from the 2019 data set and the body
        // shield must be animated.
        assert!(!spec.ground_texture.is_empty());
        assert!(!spec.ring_texture.is_empty());
        assert!(!spec.body_glow_texture.is_empty());
        assert!(spec.body_frames.len() >= 2, "body shield is not animated");
        assert!(spec.body_frames.iter().all(|frame| !frame.is_empty()));
        assert!(spec.body_frame_rate > 0.0);
        assert!(spec.body_camera_offset > 0.0);
    }

    #[test]
    fn pulse_stays_positive_and_bounded() {
        let spec = StatusAuraSpec {
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
