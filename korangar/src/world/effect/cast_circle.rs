//! Ground cast circle.
//!
//! The original client shows a hard-coded, ground-aligned circle below the
//! caster while a skill with a cast time is being cast. It is not one of the
//! `.str` effect files (those are rendered as camera-facing screen sprites),
//! which is why the old implementation looked wrong: the magic circle effect
//! was a huge billboard floating over the caster instead of a ring on the
//! ground.
//!
//! This effect draws the cast circle the same way the warp portals are drawn:
//! flat quads on the ground plane using the effect textures that exist in the
//! game files (`ring_liner.tga` for the ring, `alpha_down.tga` for the soft
//! glow). The color is chosen from the skill element, and the ring rotates
//! while the effect is alive.

use std::sync::Arc;

use cgmath::{Point3, Vector2, Vector3};

use crate::graphics::{Color, ScreenSize, Texture};
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::{EffectBase, EffectCenter, render_world_quad};

/// Height of the circle above the ground in world units. Slightly above zero so
/// that it does not z-fight with the floor.
const CIRCLE_HEIGHT: f32 = 0.2;
/// Rotation speed of the outer ring in radians per second.
const RING_ROTATION_SPEED: f32 = 1.6;
/// Rotation speed of the inner ring (opposite direction) in radians per second.
const INNER_RING_ROTATION_SPEED: f32 = -2.3;
const INNER_RING_SCALE: f32 = 0.62;
const GLOW_SCALE: f32 = 1.15;

pub struct CastCircle {
    ring_texture: Arc<Texture>,
    glow_texture: Arc<Texture>,
    center: EffectCenter,
    offset: Vector3<f32>,
    radius: f32,
    color: Color,
    time: f32,
    duration: f32,
    gets_deleted: bool,
}

impl CastCircle {
    /// Returns `None` if the game files don't contain the required effect
    /// textures, in which case the cast is simply not accompanied by a circle.
    pub fn new(
        texture_loader: &crate::loaders::TextureLoader,
        center: EffectCenter,
        radius: f32,
        color: Color,
        duration: f32,
    ) -> Option<Self> {
        let ring_texture = texture_loader
            .get_or_load("effect\\ring_liner.tga", crate::loaders::ImageType::Color)
            .ok()?;
        let glow_texture = texture_loader
            .get_or_load("effect\\alpha_down.tga", crate::loaders::ImageType::Color)
            .ok()?;

        Some(Self {
            ring_texture,
            glow_texture,
            center,
            offset: Vector3::new(0.0, CIRCLE_HEIGHT, 0.0),
            radius,
            color,
            time: 0.0,
            duration,
            gets_deleted: false,
        })
    }

    fn corners(&self, anchor: Point3<f32>, radius: f32, angle: f32) -> [Point3<f32>; 4] {
        let direction = [
            Vector3::new(-1.0, 0.0, -1.0),
            Vector3::new(1.0, 0.0, -1.0),
            Vector3::new(-1.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
        ];

        direction.map(|direction| {
            let x = direction.x * angle.cos() - direction.z * angle.sin();
            let z = direction.x * angle.sin() + direction.z * angle.cos();

            anchor + self.offset + Vector3::new(x * radius, 0.0, z * radius)
        })
    }
}

impl EffectBase for CastCircle {
    fn culling_position(&self) -> Option<Point3<f32>> {
        Some(self.center.to_position() + self.offset)
    }

    fn update(&mut self, entities: &[crate::world::Entity], delta_time: f32) -> bool {
        self.center.follow_entity(entities);
        self.time += delta_time;

        !self.gets_deleted && self.time < self.duration
    }

    fn mark_for_deletion(&mut self) {
        self.gets_deleted = true;
    }

    fn register_point_lights(&self, _: &mut crate::world::PointLightManager, _: &dyn Camera) {}

    fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera) {
        if self.gets_deleted {
            return;
        }

        let screen_size = ScreenSize {
            width: renderer.window_size().width,
            height: renderer.window_size().height,
        };

        // Fade in quickly and out towards the end of the cast so that the circle
        // does not pop.
        let fade = (self.time * 8.0).min(1.0) * ((self.duration - self.time) * 3.0).clamp(0.0, 1.0);
        let color = Color::rgba(self.color.red * fade, self.color.green * fade, self.color.blue * fade, 1.0);

        let anchor = self.center.to_position();

        // Soft ground glow below the rings.
        render_world_quad(
            renderer,
            camera,
            anchor,
            screen_size,
            self.glow_texture.clone(),
            Color::rgba(color.red * 0.14, color.green * 0.14, color.blue * 0.14, 1.0),
            self.corners(anchor, self.radius * GLOW_SCALE, 0.0),
            [
                Vector2::new(0.0, 1.0),
                Vector2::new(1.0, 1.0),
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
            ],
        );

        render_world_quad(
            renderer,
            camera,
            anchor,
            screen_size,
            self.ring_texture.clone(),
            Color::rgba(color.red * 0.55, color.green * 0.55, color.blue * 0.55, 1.0),
            self.corners(anchor, self.radius, self.time * RING_ROTATION_SPEED),
            [
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
                Vector2::new(0.0, 1.0),
                Vector2::new(1.0, 1.0),
            ],
        );

        render_world_quad(
            renderer,
            camera,
            anchor,
            screen_size,
            self.ring_texture.clone(),
            Color::rgba(color.red * 0.4, color.green * 0.4, color.blue * 0.4, 1.0),
            self.corners(anchor, self.radius * INNER_RING_SCALE, self.time * INNER_RING_ROTATION_SPEED),
            [
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
                Vector2::new(0.0, 1.0),
                Vector2::new(1.0, 1.0),
            ],
        );
    }
}

/// Color of the cast circle for the different skill properties/elements of the
/// 2019 server (`e_element`: neutral, water, earth, fire, wind, poison, holy,
/// dark, ghost, ...).
pub fn element_color(element: u32) -> Color {
    match element {
        1 => Color::rgb_u8(90, 160, 255),
        2 => Color::rgb_u8(120, 220, 120),
        3 => Color::rgb_u8(255, 120, 90),
        4 => Color::rgb_u8(150, 230, 255),
        5 => Color::rgb_u8(190, 120, 255),
        6 => Color::rgb_u8(255, 240, 180),
        7 => Color::rgb_u8(200, 130, 255),
        8 => Color::rgb_u8(200, 220, 255),
        _ => Color::rgb_u8(255, 220, 150),
    }
}

#[cfg(test)]
mod tests {
    use super::element_color;

    #[test]
    fn elements_have_distinct_cast_circle_colors() {
        // Neutral (0), water, earth, fire, wind and holy have to differ so that
        // different casts are visually distinguishable.
        let colors = [0, 1, 2, 3, 4, 6].map(element_color);
        for (index, color) in colors.iter().enumerate() {
            for other in &colors[index + 1..] {
                assert_ne!(color, other);
            }
        }
    }
}
