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

// The circle height, ring rotation speeds/scales and the ring/glow textures are
// data (`archive/data/client/skills.ron`, `cast_circle` section); this module
// only renders them.

pub struct CastCircle {
    ring_texture: Arc<Texture>,
    glow_texture: Arc<Texture>,
    /// Ring/glow geometry and animation of the data file.
    config: &'static crate::world::skills::CastCircleSpec,
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
        let config = crate::world::skills::cast_circle();
        let ring_texture = texture_loader
            .get_or_load(config.ring_texture, crate::loaders::ImageType::Color)
            .ok()?;
        let glow_texture = texture_loader
            .get_or_load(config.glow_texture, crate::loaders::ImageType::Color)
            .ok()?;

        Some(Self {
            ring_texture,
            glow_texture,
            config,
            center,
            offset: Vector3::new(0.0, config.height, 0.0),
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
            self.corners(anchor, self.radius * self.config.glow_scale, 0.0),
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
            self.corners(anchor, self.radius, self.time * self.config.ring_rotation_speed),
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
            self.corners(
                anchor,
                self.radius * self.config.inner_ring_scale,
                self.time * self.config.inner_ring_rotation_speed,
            ),
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
