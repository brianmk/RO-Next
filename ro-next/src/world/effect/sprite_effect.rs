//! Short-lived animated sprite skill effects (`.spr` + `.act`).
//!
//! The official client plays a number of skill visuals as animated sprites from
//! `data\\sprite\\이팩트\\...` (the `su_*` New World set, Soul Reaper effects,
//! Falcon Assault, several classic buffs/attacks). The layers come from the
//! unified data file `archive/data/client/skill_effects.ron` (`kind: Spr`, see
//! [`crate::world::library::skill_effects`]); every `.spr`/`.act` pair was
//! validated against the GRF archives by the data generators.
//!
//! A [`SpriteEffect`] owns its loaded [`Sprite`] and [`Actions`] (both are
//! reference-counted loader caches), plays one action once and removes itself
//! afterwards. It allocates nothing per frame:
//!
//! * the update only advances the elapsed time and follows the anchor entity,
//! * the render walks the sprite clips of the current ACT motion and submits a
//!   screen-space quad per clip through the existing [`EffectRenderer`] (the
//!   same quad pipeline the `.str` effects and billboard primitives use).
//!
//! The effect uses [`EffectBase::culling_position`] and is spawned through the
//! existing `EffectHolder`, so it obeys the global/per-entity effect budget and
//! distance culling. A missing/malformed sprite pair never panics: the spawner
//! tries the candidate list and simply spawns nothing when none is loadable,
//! leaving the generic cast circle in place.

use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Rad, Vector2, Vector3};
use wgpu::BlendFactor;

use super::{EFFECT_ORIGIN, EffectBase, EffectCenter};
use crate::graphics::Color;
use crate::loaders::{ActionLoader, Sprite, SpriteLoader};
use crate::renderer::EffectRenderer;
use crate::world::library::SkillEffectLayer;
use crate::world::{Actions, Camera};

/// Borrowed parameters of one `.spr`/`.act` sprite effect layer.
///
/// Built on the stack from a [`SkillEffectLayer`] by the skill effect spawner
/// (no allocation): the effect copies the values into its own state when it is
/// spawned.
pub(crate) struct SpriteEffectParams<'a> {
    /// Candidate sprite/action stems relative to `data\sprite\`, without the
    /// `.spr`/`.act` extension. The first pair that exists is played.
    pub files: &'a [&'a str],
    /// World-space offset from the anchor's feet.
    pub offset: Vector3<f32>,
    /// Pixel scale of the sprite (`1.0` = authored size).
    pub scale: f32,
    /// Playback duration override in milliseconds (`0` = native ACT timing).
    pub duration_ms: u32,
    /// RGB tint of the sprite.
    pub color: Color,
    /// Additive blending.
    pub additive: bool,
}

impl<'a> SpriteEffectParams<'a> {
    /// Builds the sprite parameters of a `Spr` layer.
    pub(crate) fn from_layer(layer: &'a SkillEffectLayer) -> Self {
        Self {
            files: layer.files,
            offset: layer.offset,
            scale: layer.scale,
            duration_ms: layer.duration_ms,
            color: layer.tint,
            additive: layer.additive,
        }
    }
}

/// Minimum time an ACT frame is displayed for (the file delay can be zero or
/// missing for hand-made effects).
const MIN_FRAME_TIME_MS: f32 = 1.0;
/// Upper bound for a single sprite effect, so a malformed data override can
/// never keep an effect alive forever.
const MAX_EFFECT_DURATION_MS: f32 = 30_000.0;
/// Upper bound for a single animation frame (ACT delays are in 50 ms units).
const MAX_FRAME_TIME_MS: f32 = 10_000.0;
/// Distance (in world units, one cell is 5 units) that the effect is moved
/// towards the camera for the depth test.
///
/// Entity sprites are billboards drawn with the depth of the actor, and the
/// effect pass is drawn afterwards with a depth test against that buffer. A
/// sprite effect anchored on an entity (for example the head sprite of
/// Blessing) overlaps the caster's own sprite, so its anchor has to be slightly
/// closer to the camera than the entity or the whole effect would be occluded
/// by the caster. 1.5 units is well below one cell, so walls/other actors in
/// front still occlude the effect correctly.
const CAMERA_FACING_DEPTH_OFFSET: f32 = 1.5;

/// Texture coordinates of an unmirrored quad, in the order expected by
/// [`EffectRenderer::render_effect`]: bottom-right, top-right, top-left,
/// bottom-left (see the `.str` renderer for the same order).
const QUAD_TEXTURE_COORDINATES: [Vector2<f32>; 4] = [
    Vector2::new(1.0, 1.0),
    Vector2::new(1.0, 0.0),
    Vector2::new(0.0, 0.0),
    Vector2::new(0.0, 1.0),
];
/// Mirrored (horizontally flipped) variant of [`QUAD_TEXTURE_COORDINATES`].
const MIRRORED_QUAD_TEXTURE_COORDINATES: [Vector2<f32>; 4] = [
    Vector2::new(0.0, 1.0),
    Vector2::new(0.0, 0.0),
    Vector2::new(1.0, 0.0),
    Vector2::new(1.0, 1.0),
];

/// Computes the playback timing of one sprite animation.
///
/// `frame_count` is the number of ACT motions of the played action, `delay` is
/// the ACT delay (in 50 ms units) and `duration_override_ms` is the data-file
/// override (`0` = native timing). Returns `(frame_time_ms,
/// total_duration_ms)`.
pub(crate) fn playback_timing(frame_count: usize, delay: f32, duration_override_ms: u32) -> (f32, f32) {
    let frame_count = frame_count.max(1) as f32;
    let native_frame_time = (delay * 50.0).clamp(MIN_FRAME_TIME_MS, MAX_FRAME_TIME_MS);

    match duration_override_ms {
        0 => (native_frame_time, (native_frame_time * frame_count).min(MAX_EFFECT_DURATION_MS)),
        duration => {
            let duration = (duration as f32).clamp(MIN_FRAME_TIME_MS, MAX_EFFECT_DURATION_MS);
            (duration / frame_count, duration)
        }
    }
}

/// One animated `.spr`/`.act` skill effect.
pub(crate) struct SpriteEffect {
    sprite: Arc<Sprite>,
    actions: Arc<Actions>,
    center: EffectCenter,
    offset: Vector3<f32>,
    scale: f32,
    color: Color,
    additive: bool,
    /// Milliseconds per animation frame.
    frame_time: f32,
    frame_count: usize,
    /// Total playback duration in milliseconds.
    duration: f32,
    elapsed: f32,
}

impl SpriteEffect {
    /// Tries the candidate sprite pairs of `spec` and returns the first one
    /// that loads; `None` when none of them exists/parses (the caller then
    /// keeps the generic cast circle).
    pub(crate) fn spawn(
        sprite_loader: &SpriteLoader,
        action_loader: &ActionLoader,
        center: EffectCenter,
        spec: &SpriteEffectParams<'_>,
    ) -> Option<Self> {
        for stem in spec.files {
            let sprite_file = format!("{stem}.spr");
            let action_file = format!("{stem}.act");

            // Check both files first: the individual loaders fall back to a
            // black placeholder sprite / the fallback actions when a file is
            // missing, which must not mask a missing candidate.
            if !sprite_loader.file_exists(&sprite_file) || !action_loader.file_exists(&action_file) {
                continue;
            }

            let Ok(sprite) = sprite_loader.get_or_load(&sprite_file, None) else {
                continue;
            };
            let Ok(actions) = action_loader.get_or_load(&action_file) else {
                continue;
            };

            if let Some(effect) = Self::from_loaded(sprite, actions, center.clone(), spec) {
                return Some(effect);
            }
        }

        None
    }

    fn from_loaded(sprite: Arc<Sprite>, actions: Arc<Actions>, center: EffectCenter, spec: &SpriteEffectParams<'_>) -> Option<Self> {
        let action = actions.actions.first()?;
        let frame_count = action.motions.len();

        if frame_count == 0 || sprite.textures.is_empty() {
            return None;
        }

        let delay = actions.delays.first().copied().unwrap_or(0.0);
        let (frame_time, duration) = playback_timing(frame_count, delay, spec.duration_ms);

        if !frame_time.is_finite() || frame_time <= 0.0 || !duration.is_finite() || duration <= 0.0 {
            return None;
        }

        Some(Self {
            sprite,
            actions,
            center,
            offset: spec.offset,
            scale: if spec.scale > 0.0 { spec.scale } else { 1.0 },
            color: Color::rgba(spec.color.red, spec.color.green, spec.color.blue, 1.0),
            additive: spec.additive,
            frame_time,
            frame_count,
            duration,
            elapsed: 0.0,
        })
    }

    fn current_frame(&self) -> usize {
        ((self.elapsed / self.frame_time) as usize).min(self.frame_count.saturating_sub(1))
    }

    fn blend_factors(&self) -> (BlendFactor, BlendFactor) {
        // Sprite textures are premultiplied by `SpriteLoader`, so the quad is
        // submitted with the premultiplied factors directly. The forward effect
        // pass only rewrites a straight-alpha `SrcAlpha` source factor; `One` is
        // already the premultiplied form and is passed through unchanged.
        match self.additive {
            true => (BlendFactor::One, BlendFactor::One),
            false => (BlendFactor::One, BlendFactor::OneMinusSrcAlpha),
        }
    }
}

impl EffectBase for SpriteEffect {
    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.center.to_position() + self.offset)
    }

    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        if let EffectCenter::Entity(entity_id, position) = &mut self.center
            && let Some(entity) = entities.iter().find(|entity| entity.get_entity_id() == *entity_id)
        {
            *position = entity.get_position();
        }

        self.elapsed += (delta_time * 1000.0).max(0.0);

        self.elapsed < self.duration
    }

    fn mark_for_deletion(&mut self) {
        self.elapsed = self.duration;
    }

    fn register_point_lights(&self, _point_light_manager: &mut crate::world::PointLightManager, _camera: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let anchor = self.center.to_position() + self.offset;

        // Pull the anchor slightly towards the camera so the effect is not
        // occluded by the actor it is attached to (see
        // `CAMERA_FACING_DEPTH_OFFSET`).
        let towards_camera = camera.camera_position() - anchor;
        let position = match towards_camera.magnitude2() > f32::EPSILON {
            true => anchor + towards_camera.normalize() * CAMERA_FACING_DEPTH_OFFSET,
            false => anchor,
        };

        let Some(action) = self.actions.actions.first() else {
            return;
        };
        let Some(motion) = action.motions.get(self.current_frame()) else {
            return;
        };

        let (source_blend_factor, destination_blend_factor) = self.blend_factors();

        for sprite_clip in &motion.sprite_clips {
            if sprite_clip.sprite_number < 0 {
                continue;
            }

            let mut sprite_number = sprite_clip.sprite_number as usize;
            // The sprite type is 0 for palette images and 1 for BGRA images.
            let sprite_type = sprite_clip.sprite_type.unwrap_or(0) as usize;

            if sprite_type == 1 {
                sprite_number += self.sprite.palette_size;
            }

            let Some(texture) = self.sprite.textures.get(sprite_number) else {
                continue;
            };

            let texture_size = texture.get_size();
            let dimensions: Vector2<f32> = sprite_clip.size.map_or_else(
                || Vector2::new(texture_size.width as f32, texture_size.height as f32),
                |size| Vector2::new(size.x as f32, size.y as f32),
            );

            let zoom = sprite_clip.zoom.unwrap_or(1.0) * self.scale;
            let zoom2 = sprite_clip.zoom2.unwrap_or_else(|| Vector2::new(1.0, 1.0));
            let dimensions = dimensions.zip(zoom2, |dimension, zoom| dimension * zoom) * zoom;

            let half_size = dimensions / 2.0;
            let angle = Rad((sprite_clip.angle.unwrap_or(0) as f32).to_radians());
            let (sin, cos) = angle.0.sin_cos();

            // Quad corners in the order the effect renderer expects them:
            // [top-left, top-right, bottom-left, bottom-right]. They are local
            // pixel offsets from the anchor: the effect renderer projects the
            // world anchor itself and adds that screen position (the same
            // convention as the `.str` effects and `render_world_quad`).
            let local_corners = [
                Vector2::new(-half_size.x, -half_size.y),
                Vector2::new(half_size.x, -half_size.y),
                Vector2::new(-half_size.x, half_size.y),
                Vector2::new(half_size.x, half_size.y),
            ];
            let center = Vector2::new(sprite_clip.position.x as f32, sprite_clip.position.y as f32);

            let corner_screen_position = local_corners.map(|corner| {
                let rotated = Vector2::new(corner.x * cos - corner.y * sin, corner.x * sin + corner.y * cos);
                EFFECT_ORIGIN + center + rotated
            });

            let texture_coordinates = match sprite_clip.mirror_on != 0 {
                true => MIRRORED_QUAD_TEXTURE_COORDINATES,
                false => QUAD_TEXTURE_COORDINATES,
            };

            renderer.render_effect(
                camera,
                position,
                texture.clone(),
                corner_screen_position,
                texture_coordinates,
                Vector2::new(0.0, 0.0),
                Rad(0.0),
                self.color,
                source_blend_factor,
                destination_blend_factor,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_EFFECT_DURATION_MS, playback_timing};

    #[test]
    fn native_timing_uses_the_act_delay() {
        // 12 frames with an ACT delay of 3.0 => 12 * (3 * 50) = 1800 ms.
        let (frame_time, duration) = playback_timing(12, 3.0, 0);
        assert_eq!(frame_time, 150.0);
        assert_eq!(duration, 1800.0);
    }

    #[test]
    fn duration_override_stretches_the_animation() {
        let (frame_time, duration) = playback_timing(10, 2.0, 1500);
        assert_eq!(frame_time, 150.0);
        assert_eq!(duration, 1500.0);
    }

    #[test]
    fn degenerate_act_delays_do_not_break_the_timing() {
        // A missing/zero ACT delay falls back to 1 ms per frame.
        let (frame_time, duration) = playback_timing(4, 0.0, 0);
        assert_eq!(frame_time, 1.0);
        assert_eq!(duration, 4.0);
    }

    #[test]
    fn duration_is_bounded() {
        let (_, duration) = playback_timing(1, 1.0, u32::MAX);
        assert_eq!(duration, MAX_EFFECT_DURATION_MS);
        // A pathological ACT delay is clamped to the maximum frame time.
        let (frame_time, _) = playback_timing(2, 1_000.0, 0);
        assert_eq!(frame_time, 10_000.0);
    }

    #[test]
    fn zero_frames_are_treated_as_one() {
        let (frame_time, duration) = playback_timing(0, 2.0, 0);
        assert_eq!(frame_time, 100.0);
        assert_eq!(duration, 100.0);
    }
}
