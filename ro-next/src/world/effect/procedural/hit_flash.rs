//! Hit flash / billboard burst primitive.
//!
//! The official client plays `2D`/`3D` effect-table components at a target when
//! a hit lands (the standard `EF_HIT`, element hit flashes like `EF_FIREHIT`,
//! and the small quad bursts of many damage skills). This primitive reproduces
//! them with a fixed number of camera-facing textured quads that
//! expand/contract and fade out at the target.

use std::sync::Arc;

use cgmath::{Point3, Vector3};

use super::{FULL_QUAD_TEXTURE_COORDINATES, GOLDEN_ANGLE, blend_factors, camera_facing_corners, pseudo_random, raised};
use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::procedural::PrimitiveParams;
use crate::world::effect::{EffectBase, EffectCenter, render_world_quad_blended};

/// Fraction of the lifetime used to fade the burst in / out.
const FADE_IN: f32 = 0.12;
const FADE_OUT: f32 = 0.35;

/// A short burst of billboard quads around a position or entity.
pub struct HitFlash {
    texture: Arc<Texture>,
    center: EffectCenter,
    /// Seconds to wait before the first frame (data-file `delay_ms`).
    delay: f32,
    /// Extra world offset (above the feet `height` included) for
    /// entity-attached layers; `EffectCenter::follow_entity` only refreshes
    /// the entity position, so the offset has to be kept here or it would
    /// be lost every frame.
    offset: Vector3<f32>,
    height: f32,
    count: u16,
    duration: f32,
    size: f32,
    size_end: f32,
    spread: f32,
    alpha: f32,
    color: Color,
    additive: bool,
    time: f32,
    seed: u32,
    gets_deleted: bool,
}

impl HitFlash {
    /// Spawns the primitive for one data-file row. Returns `None` if the effect
    /// texture is not part of the game data.
    pub fn spawn(
        texture_loader: &TextureLoader,
        center: EffectCenter,
        offset: Vector3<f32>,
        delay_ms: u32,
        spec: &PrimitiveParams<'_>,
    ) -> Option<Self> {
        let texture = texture_loader.get_or_load(spec.texture, ImageType::Color).ok()?;

        Some(Self {
            texture,
            center,
            offset,
            delay: delay_ms as f32 / 1000.0,
            height: spec.height,
            count: spec.count.clamp(1, 8),
            duration: (spec.duration_ms.max(50) as f32) / 1000.0,
            size: spec.size.max(1.0),
            size_end: spec.size_end.max(1.0),
            spread: spec.spread,
            alpha: spec.alpha.clamp(0.0, 1.0),
            color: Color::rgba(spec.color.red, spec.color.green, spec.color.blue, 1.0),
            additive: spec.additive,
            time: 0.0,
            seed: spec.seed,
            gets_deleted: false,
        })
    }

    /// Current fade weight of the burst (0..1).
    fn fade(&self) -> f32 {
        fade_weight(self.time - self.delay, self.duration)
    }
}

/// Fade in at the start and out at the end of the burst lifetime.
fn fade_weight(elapsed: f32, duration: f32) -> f32 {
    let progress = (elapsed / duration.max(f32::EPSILON)).clamp(0.0, 1.0);
    let fade_in = (progress / FADE_IN).clamp(0.0, 1.0);
    let fade_out = ((1.0 - progress) / FADE_OUT).clamp(0.0, 1.0);
    fade_in.min(fade_out)
}

impl EffectBase for HitFlash {
    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(raised(self.center.to_position() + self.offset, self.height))
    }

    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.center.follow_entity(entities);
        self.time += delta_time;
        !self.gets_deleted && self.time < self.delay + self.duration
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut crate::world::PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        if self.time < self.delay {
            return;
        }

        let fade = self.fade();
        if fade <= 0.001 {
            return;
        }

        let progress = ((self.time - self.delay) / self.duration).clamp(0.0, 1.0);
        let size = self.size + (self.size_end - self.size) * progress;
        let alpha = self.alpha * fade;
        let color = Color::rgba(self.color.red, self.color.green, self.color.blue, alpha);
        let (source_blend_factor, destination_blend_factor) = blend_factors(self.additive);
        let screen_size = ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        };
        let anchor = raised(self.center.to_position() + self.offset, self.height);

        for index in 0..self.count {
            let seed = self.seed ^ (index as u32).wrapping_mul(0x9E37_79B9);
            let angle = index as f32 * GOLDEN_ANGLE;
            let radius = self.spread * pseudo_random(seed);
            let rise = self.spread * 0.5 * (pseudo_random(seed ^ 0x5555_5555) - 0.3);
            let position = anchor + Vector3::new(angle.cos() * radius, rise, angle.sin() * radius);
            // Textured skill glyphs (Asura kanji, ...) are upright and must not
            // spin; only spread-out spark bursts rotate over time.
            let spin = if self.spread > 0.0 { angle + self.time * 1.5 } else { angle };
            let half_size = size * 0.5 * (0.85 + 0.3 * pseudo_random(seed ^ 0x1234_5678));
            let corners = camera_facing_corners(camera, position, half_size, spin);

            render_world_quad_blended(
                renderer,
                camera,
                position,
                screen_size,
                self.texture.clone(),
                color,
                corners,
                FULL_QUAD_TEXTURE_COORDINATES,
                source_blend_factor,
                destination_blend_factor,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::fade_weight;

    #[test]
    fn fade_starts_and_ends_transparent() {
        assert_eq!(fade_weight(0.0, 0.3), 0.0);
        assert!(fade_weight(0.15, 0.3) > 0.9, "the middle of the burst is fully visible");
        assert_eq!(fade_weight(0.3, 0.3), 0.0);
        assert_eq!(fade_weight(0.5, 0.3), 0.0);
    }
}
