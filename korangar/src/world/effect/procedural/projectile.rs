//! Projectile / bolt-rain primitive.
//!
//! The official client simulates bolts and arrows in code (`FUNC` components:
//! `BoltRain`, arrow/spear projectiles, thrown fireballs). This primitive
//! covers the two common shapes:
//!
//! * [`ProjectileMode::Travel`] - a small number of textured quads flying from
//!   the caster to the target along a slight arc, each with its own start
//!   delay, followed by an impact flash at the target.
//! * [`ProjectileMode::Rain`] - a burst of bolts falling from above onto the
//!   target cell (cold/fire bolt style `BoltRain`), followed by an impact
//!   flash.

use std::sync::Arc;

use cgmath::{Point3, Vector3};

use super::{FULL_QUAD_TEXTURE_COORDINATES, GOLDEN_ANGLE, blend_factors, camera_facing_corners, pseudo_random};
use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::{EffectBase, render_world_quad_blended};
use crate::world::library::ProceduralEffectSpec;

/// How long the impact flash stays visible after a projectile lands.
const IMPACT_DURATION: f32 = 0.2;
/// Height of the arc of a travelling projectile in world units.
const ARC_HEIGHT: f32 = 3.0;
/// Height of the target impact point above the feet.
const TARGET_HEIGHT: f32 = 4.0;

/// The two projectile shapes of the official client simulations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectileMode {
    /// Fly from the caster to the target.
    Travel,
    /// Fall from above onto the target cell.
    Rain,
}

/// A group of textured quads flying towards (or falling onto) a target.
pub struct Projectile {
    texture: Arc<Texture>,
    mode: ProjectileMode,
    start: Point3<f32>,
    end: Point3<f32>,
    count: u16,
    duration: f32,
    stagger: f32,
    size: f32,
    size_end: f32,
    spread: f32,
    height: f32,
    alpha: f32,
    color: Color,
    additive: bool,
    time: f32,
    seed: u32,
    gets_deleted: bool,
}

/// Count, per-projectile travel time and start delay of one data-file row.
fn timing(spec: &ProceduralEffectSpec, mode: ProjectileMode) -> (u16, f32, f32) {
    let count = spec.count.clamp(1, 10);
    let duration = (spec.duration_ms.max(50) as f32) / 1000.0;
    let stagger = match (spec.delay_ms, mode) {
        (0, ProjectileMode::Rain) => duration / count as f32,
        (delay_ms, _) => (delay_ms as f32) / 1000.0,
    };
    (count, duration, stagger)
}

/// Total lifetime of a projectile group.
fn total_duration_of(duration: f32, stagger: f32, count: u16) -> f32 {
    duration + stagger * (count.saturating_sub(1) as f32) + IMPACT_DURATION
}

impl Projectile {
    /// Spawns the primitive for one data-file row. Returns `None` if the effect
    /// texture is not part of the game data.
    pub fn spawn(
        texture_loader: &TextureLoader,
        spec: &ProceduralEffectSpec,
        mode: ProjectileMode,
        start: Point3<f32>,
        end: Point3<f32>,
    ) -> Option<Self> {
        let texture = texture_loader.get_or_load(&spec.texture, ImageType::Color).ok()?;
        let (count, duration, stagger) = timing(spec, mode);

        Some(Self {
            texture,
            mode,
            start,
            end,
            count,
            duration,
            stagger,
            size: spec.size.max(1.0),
            size_end: spec.size_end.max(1.0),
            spread: spec.spread,
            height: spec.height,
            alpha: spec.alpha.clamp(0.0, 1.0),
            color: Color::rgba(spec.color.0, spec.color.1, spec.color.2, 1.0),
            additive: spec.additive,
            time: 0.0,
            seed: spec.skill as u32 * 6151 + spec.texture.len() as u32,
            gets_deleted: false,
        })
    }

    /// Total lifetime including the last staggered projectile and the impact.
    fn total_duration(&self) -> f32 {
        total_duration_of(self.duration, self.stagger, self.count)
    }

    /// Position of the target impact point.
    fn target_point(&self) -> Point3<f32> {
        self.end + Vector3::new(0.0, TARGET_HEIGHT, 0.0)
    }

    /// Renders one quad at `position`.
    fn render_quad(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, position: Point3<f32>, size: f32, angle: f32, alpha: f32) {
        if alpha <= 0.001 {
            return;
        }

        let screen_size = ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        };
        let (source_blend_factor, destination_blend_factor) = blend_factors(self.additive);
        let color = Color::rgba(self.color.red, self.color.green, self.color.blue, alpha);
        let corners = camera_facing_corners(camera, position, size * 0.5, angle);

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

    /// Renders the impact flash at the target while a landed projectile fades.
    fn render_impact(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, elapsed: f32) {
        if !(0.0..=IMPACT_DURATION).contains(&elapsed) {
            return;
        }

        let alpha = self.alpha * (1.0 - elapsed / IMPACT_DURATION);
        let size = self.size_end.max(self.size) * 1.8;
        self.render_quad(renderer, camera, self.target_point(), size, elapsed * 2.0, alpha);
    }

    fn render_travel(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        for index in 0..self.count {
            let delay = self.stagger * index as f32;
            let progress = (self.time - delay) / self.duration;

            if progress <= 0.0 || progress >= 1.0 {
                if progress >= 1.0 && self.time - delay - self.duration < IMPACT_DURATION {
                    self.render_impact(renderer, camera, self.time - delay - self.duration);
                }
                continue;
            }

            let start = self.start + Vector3::new(0.0, self.height, 0.0);
            let end = self.target_point();
            let arc = (progress * std::f32::consts::PI).sin() * ARC_HEIGHT;
            let position = start + (end - start) * progress + Vector3::new(0.0, arc, 0.0);
            let alpha = self.alpha * (0.4 + 0.6 * progress.min(0.9));
            let size = self.size + (self.size_end - self.size) * progress;

            self.render_quad(renderer, camera, position, size, progress * 6.0, alpha);
        }
    }

    fn render_rain(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        for index in 0..self.count {
            let delay = self.stagger * index as f32;
            let progress = (self.time - delay) / self.duration;

            if progress >= 1.0 {
                let elapsed = self.time - delay - self.duration;
                if elapsed < IMPACT_DURATION {
                    self.render_impact(renderer, camera, elapsed);
                }
                continue;
            }

            if progress <= 0.0 {
                continue;
            }

            let seed = self.seed ^ (index as u32).wrapping_mul(0x9E37_79B9);
            let angle = index as f32 * GOLDEN_ANGLE;
            let radius = self.spread * (0.3 + 0.7 * pseudo_random(seed));
            let offset = Vector3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
            let position = self.end + offset + Vector3::new(0.0, TARGET_HEIGHT + self.height * (1.0 - progress), 0.0);
            let alpha = self.alpha * (0.5 + 0.5 * progress);

            self.render_quad(renderer, camera, position, self.size, index as f32, alpha);
        }
    }
}

impl EffectBase for Projectile {
    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.target_point())
    }

    fn update(&mut self, _entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.time += delta_time;
        !self.gets_deleted && self.time < self.total_duration()
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut crate::world::PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        match self.mode {
            ProjectileMode::Travel => self.render_travel(renderer, camera),
            ProjectileMode::Rain => self.render_rain(renderer, camera),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IMPACT_DURATION, ProjectileMode, timing, total_duration_of};
    use crate::world::library::{ProceduralAnchor, ProceduralEffectSpec, ProceduralPrimitive, SkillEffectVariant};

    fn spec(delay_ms: u32, count: u16, duration_ms: u32) -> ProceduralEffectSpec {
        ProceduralEffectSpec {
            skill: 14,
            variant: SkillEffectVariant::Hit,
            anchor: ProceduralAnchor::Rain,
            primitive: ProceduralPrimitive::Projectile,
            texture: "effect\\icearrow.tga".to_owned(),
            count,
            duration_ms,
            delay_ms,
            size: 10.0,
            size_end: 8.0,
            spread: 16.0,
            height: 40.0,
            bottom_radius: 0.0,
            top_radius: 0.0,
            alpha: 0.9,
            color: (1.0, 1.0, 1.0),
            additive: true,
            rotation_speed: 0.0,
        }
    }

    #[test]
    fn rain_staggers_the_bolts_over_the_lifetime() {
        let (count, duration, stagger) = timing(&spec(0, 6, 600), ProjectileMode::Rain);
        assert_eq!(count, 6);
        assert!((stagger - 0.1).abs() < 1e-5);
        assert!((total_duration_of(duration, stagger, count) - (0.6 + 0.5 + IMPACT_DURATION)).abs() < 1e-5);
    }

    #[test]
    fn travel_uses_the_explicit_delay_between_projectiles() {
        let (count, duration, stagger) = timing(&spec(333, 1, 333), ProjectileMode::Travel);
        assert_eq!(count, 1);
        assert!((stagger - 0.333).abs() < 1e-5);
        assert!((total_duration_of(duration, stagger, count) - (0.333 + IMPACT_DURATION)).abs() < 1e-5);
    }
}
