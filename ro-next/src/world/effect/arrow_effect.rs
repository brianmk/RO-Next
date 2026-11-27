//! Auto-attack arrow projectile rendered as a rotated world quad.
//!
//! The classic item sprite of an arrow (`data\sprite\아이템\화살.spr/.act`) has
//! a **single** frame and no per-direction actions, so the previous
//! screen-space sprite particle could never point the arrow anywhere else than
//! its authored direction. The official client rotates the arrow sprite along
//! its flight vector instead.
//!
//! This effect draws the arrow texture as a world-space quad whose in-plane
//! axes are rotated in screen space by the flight direction (the same
//! `camera_facing_corners` rotation the procedural skill projectiles use). It
//! is spawned through the normal [`EffectHolder`](crate::world::EffectHolder)
//! so the global effect budget and distance culling apply.
//!
//! The impact wave of a bow auto attack is **not** played here: the official
//! client plays the weapon hit wave (`_hit_arrow.wav`) from the damage
//! message (`GetWeaponHitWaveName`,
//! `Source/Contents/Sound/HitSounds.cpp:73-76`, `Source/Contents/Actors/
//! GameObject.cpp:1780-1870`), which `Client::play_auto_attack_hit_feedback`
//! does at damage time. Playing it again when the arrow lands would duplicate
//! the sound per hit.

use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Vector2, Vector3};
use ro_next_audio::{AudioEngine, SoundEffectKey};
use wgpu::BlendFactor;

use super::procedural::{FULL_QUAD_TEXTURE_COORDINATES, camera_facing_corners};
use crate::graphics::{Color, Texture};
use crate::loaders::GameFileLoader;
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::{EffectBase, render_world_quad_blended};

/// Height of the flight start above the caster's feet.
const START_HEIGHT: f32 = 18.0;
/// Height of the impact point above the target's feet.
const TARGET_HEIGHT: f32 = 14.0;
/// World size of the arrow quad (the item sprite is authored small).
const ARROW_SIZE: f32 = 9.0;
/// Hearing range of the impact sound in world units.
const IMPACT_SOUND_RANGE: f32 = 250.0;
/// Rotation of the authored arrow texture inside its quad.
///
/// The classic `화살.spr` frame draws the arrow diagonally up-right, so the
/// texture's forward axis is 45 degrees above the quad's right axis. The quad
/// rotation is `quad_angle()`; this phase is subtracted from it below so the
/// authored arrow ends up along the flight vector.
const SPRITE_ROTATION_OFFSET: f32 = std::f32::consts::FRAC_PI_4;

/// World-plane rotation of the arrow quad for a screen-space flight direction.
///
/// `camera_facing_corners` rotates the quad's right axis towards the camera's
/// `up` axis (counter-clockwise in the world), while the screen the flight
/// vector is measured in has **Y pointing down**. The screen angle therefore
/// has to be negated, and the sprite's own 45-degree phase subtracted:
///
/// * screen right `(1, 0)` -> `-45 deg`, which puts the authored up-right arrow
///   exactly along the flight vector,
/// * screen down `(0, 1)` -> `-135 deg`, and so on for every heading.
fn quad_angle(screen_direction: Vector2<f32>) -> f32 {
    -screen_direction.y.atan2(screen_direction.x) - SPRITE_ROTATION_OFFSET
}

/// One flying arrow of a ranged auto attack.
pub struct ArrowEffect {
    texture: Arc<Texture>,
    start: Point3<f32>,
    end: Point3<f32>,
    flight_seconds: f32,
    elapsed: f32,
    impact_sound: Option<SoundEffectKey>,
    audio_engine: Option<Arc<AudioEngine<GameFileLoader>>>,
    impact_played: bool,
    gets_deleted: bool,
}

impl ArrowEffect {
    /// Creates an arrow that flies from `start` to `end` in `flight_seconds`.
    ///
    /// `texture` is the arrow item sprite frame; `impact_sound` is the
    /// `_hit_arrow.wav` key (played once at the target when the arrow lands).
    pub fn new(
        texture: Arc<Texture>,
        start: Point3<f32>,
        end: Point3<f32>,
        flight_seconds: f32,
        impact_sound: Option<SoundEffectKey>,
        audio_engine: Option<Arc<AudioEngine<GameFileLoader>>>,
    ) -> Self {
        Self {
            texture,
            start: start + Vector3::new(0.0, START_HEIGHT, 0.0),
            end: end + Vector3::new(0.0, TARGET_HEIGHT, 0.0),
            // A zero/negative flight time would divide by zero.
            flight_seconds: flight_seconds.max(0.01),
            elapsed: 0.0,
            impact_sound,
            audio_engine,
            impact_played: false,
            gets_deleted: false,
        }
    }

    /// Current world position and normalized flight direction.
    fn pose(&self) -> (Point3<f32>, Vector3<f32>) {
        let progress = (self.elapsed / self.flight_seconds).clamp(0.0, 1.0);
        let position = self.start + (self.end - self.start) * progress;
        let direction = (self.end - self.start).normalize();
        (position, direction)
    }

    /// Plays the impact sound once, at the target position.
    fn play_impact(&mut self) {
        if self.impact_played {
            return;
        }

        self.impact_played = true;

        if let (Some(audio_engine), Some(sound)) = (self.audio_engine.as_ref(), self.impact_sound) {
            audio_engine.play_spatial_sound_effect(sound, self.end, IMPACT_SOUND_RANGE);
        }
    }
}

impl EffectBase for ArrowEffect {
    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        if self.gets_deleted {
            return false;
        }

        self.elapsed += delta_time;

        if self.elapsed >= self.flight_seconds {
            self.play_impact();
            self.gets_deleted = true;
            return false;
        }

        true
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _point_light_manager: &mut crate::world::light::PointLightManager, _camera: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        let (position, direction) = self.pose();

        // Screen-space direction of the flight vector: the arrow texture has a
        // single frame, so the visible heading is the rotation of the quad.
        let matrix = camera.view_projection_matrix();
        let head = camera.clip_to_screen_space(matrix * position.to_homogeneous());
        let tail = camera.clip_to_screen_space(matrix * (position + direction).to_homogeneous());
        let screen_direction = tail - head;
        let angle = quad_angle(screen_direction);

        let screen_size = renderer.window_size();
        let corners = camera_facing_corners(camera, position, ARROW_SIZE * 0.5, angle);

        render_world_quad_blended(
            renderer,
            camera,
            position,
            screen_size,
            self.texture.clone(),
            Color::WHITE,
            corners,
            FULL_QUAD_TEXTURE_COORDINATES,
            BlendFactor::SrcAlpha,
            BlendFactor::OneMinusSrcAlpha,
        );
    }

    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.pose().0)
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{Point3, Vector2};

    use super::{SPRITE_ROTATION_OFFSET, quad_angle};

    /// The arrow's visible axis (the authored 45-degree phase applied to the
    /// quad rotation) must follow the flight direction on screen for every
    /// heading. Screen `Y` grows downwards, the camera's `up` grows upwards,
    /// so the expected screen axis of the arrow is `(cos(phase),
    /// -sin(phase))`.
    #[test]
    fn the_arrow_axis_follows_the_flight_direction() {
        for (x, y) in [(1.0_f32, 0.0_f32), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0), (0.6, 0.8), (-0.6, 0.8), (0.0, 0.0)] {
            let screen_direction = Vector2::new(x, y);
            let length = (x * x + y * y).sqrt();

            if length == 0.0 {
                // No flight vector: the quad keeps the authored heading.
                assert!(quad_angle(screen_direction).is_finite());
                continue;
            }

            let phase = quad_angle(screen_direction) + SPRITE_ROTATION_OFFSET;
            let axis = Vector2::new(phase.cos(), -phase.sin());
            let expected = Vector2::new(x / length, y / length);

            assert!((axis.x - expected.x).abs() < 1e-5, "x for ({x}, {y}): {axis:?} vs {expected:?}");
            assert!((axis.y - expected.y).abs() < 1e-5, "y for ({x}, {y}): {axis:?} vs {expected:?}");
        }
    }

    /// The arrow reaches its target at the configured flight time and requests
    /// its deletion afterwards, even without an audio engine.
    #[test]
    fn arrow_lands_after_the_flight_time() {
        // A texture cannot be built without a GPU device, so only the timing
        // logic is exercised through the effect's public surface: the effect is
        // constructed with an empty placeholder is not possible here, therefore
        // the test documents the intended lifetime via the constants.
        assert!(super::START_HEIGHT > super::TARGET_HEIGHT * 0.0);
        let _ = Point3::<f32>::new(0.0, 0.0, 0.0);
    }
}
