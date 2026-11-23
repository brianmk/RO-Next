use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Vector2, Vector3};

use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, SpriteLoader, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;
use crate::world::effect::render_world_quad;

/// The map connection portal is not a sprite: the official client renders a
/// hard-coded effect made out of the `alpha_down.tga` and `ring_blue.tga`
/// effect textures plus the `particle1` sparkle sprite. We recreate that effect
/// so that warps (`JT_WARPNPC`) are visible again instead of falling back to
/// the black "missing sprite" placeholder.
const CYLINDER_RADIUS: f32 = 7.0;
const CYLINDER_HEIGHT: f32 = 24.0;
const CYLINDER_SEGMENT_COUNT: usize = 16;
const CYLINDER_TEXTURE_REPEAT: f32 = 2.0;
const GROUND_GLOW_RADIUS: f32 = 14.0;
const GROUND_GLOW_HEIGHT: f32 = 0.15;
const PARTICLE_COUNT: usize = 12;
const PARTICLE_HEIGHT: f32 = 26.0;
const PARTICLE_MIN_SIZE: f32 = 2.2;
const PARTICLE_SIZE_VARIATION: f32 = 1.4;
const MAXIMUM_DISTANCE: f32 = 600.0;
// The textures of both portal styles and the flat skill-unit portal radius /
// height / ring count are data (`archive/data/client/skills.ron`,
// `warp_portal` section). The cylinder/glow/particle geometry below is the
// procedural shape of the map-connection warp and stays code.

/// Textures used by the warp portal effect. They are loaded once when the
/// client starts, because they are shared between all warps on the map.
#[derive(Clone)]
pub struct WarpPortal {
    streak_texture: Arc<Texture>,
    gradient_texture: Arc<Texture>,
    particle_texture: Arc<Texture>,
    /// Textures and flat skill-unit portal geometry of the data file.
    config: &'static crate::world::skills::WarpPortalSpec,
}

impl WarpPortal {
    /// Returns `None` if the game files don't contain the effect textures, in
    /// which case warps simply stay invisible.
    pub fn new(texture_loader: &TextureLoader, sprite_loader: &SpriteLoader) -> Option<Self> {
        let config = crate::world::skills::warp_portal();
        let streak_texture = texture_loader.get_or_load(config.streak_texture, ImageType::Color).ok()?;
        let gradient_texture = texture_loader.get_or_load(config.gradient_texture, ImageType::Color).ok()?;
        let particle_sprite = sprite_loader.get_or_load(config.particle_sprite, None).ok()?;
        let particle_texture = particle_sprite.textures.first()?.clone();

        Some(Self {
            streak_texture,
            gradient_texture,
            particle_texture,
            config,
        })
    }

    /// Renders the map connection warp (`JT_WARPNPC`): a rotating cylinder with
    /// a ground glow and rising particles.
    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, position: Point3<f32>, screen_size: ScreenSize, time: f32) {
        if (camera.focus_point() - position).magnitude() > MAXIMUM_DISTANCE {
            return;
        }

        let anchor = position;
        let rotation = time * 0.15;
        let scroll = (time * 0.55) % 1.0;

        self.render_cylinder(
            renderer,
            camera,
            anchor,
            screen_size,
            self.streak_texture.clone(),
            Color::rgba(0.20, 0.40, 0.82, 1.0),
            rotation,
            scroll,
        );

        self.render_cylinder(
            renderer,
            camera,
            anchor,
            screen_size,
            self.gradient_texture.clone(),
            Color::rgba(0.12, 0.24, 0.46, 1.0),
            rotation,
            0.0,
        );

        self.render_ground_glow(renderer, camera, anchor, screen_size, time);
        self.render_particles(renderer, camera, anchor, screen_size, time);
    }

    /// Renders an open skill-unit warp portal (AL_WARP). The original client
    /// draws it as a flat, rotating swirl on the ground with a soft glow and a
    /// few rising sparkles, so this is a flat effect instead of the map warp
    /// cylinder.
    pub fn render_skill_unit(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        position: Point3<f32>,
        screen_size: ScreenSize,
        time: f32,
        fade: f32,
    ) {
        if (camera.focus_point() - position).magnitude() > MAXIMUM_DISTANCE || fade <= 0.0 {
            return;
        }

        // The skill-unit portal uses the same visible animated warp effect as
        // the map-connection warps (rotating cylinder + ground glow + rising
        // sparks). The previous flat rings were too low/subtle and could be
        // occluded by the ground, which made AL_WARP look like it had no
        // animation at all.
        self.render(renderer, camera, position, screen_size, time);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_cylinder(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        anchor: Point3<f32>,
        screen_size: ScreenSize,
        texture: Arc<Texture>,
        color: Color,
        rotation: f32,
        scroll: f32,
    ) {
        let top_offset = Vector3::new(0.0, CYLINDER_HEIGHT, 0.0);
        let segment_angle = TAU / CYLINDER_SEGMENT_COUNT as f32;

        for segment in 0..CYLINDER_SEGMENT_COUNT {
            let angle_start = rotation + segment as f32 * segment_angle;
            let angle_end = rotation + (segment + 1) as f32 * segment_angle;

            let bottom_start = anchor + Vector3::new(angle_start.cos() * CYLINDER_RADIUS, 0.0, angle_start.sin() * CYLINDER_RADIUS);
            let bottom_end = anchor + Vector3::new(angle_end.cos() * CYLINDER_RADIUS, 0.0, angle_end.sin() * CYLINDER_RADIUS);

            let texture_start = segment as f32 / CYLINDER_SEGMENT_COUNT as f32 * CYLINDER_TEXTURE_REPEAT;
            let texture_end = (segment + 1) as f32 / CYLINDER_SEGMENT_COUNT as f32 * CYLINDER_TEXTURE_REPEAT;

            render_world_quad(
                renderer,
                camera,
                anchor,
                screen_size,
                texture.clone(),
                color,
                [bottom_start + top_offset, bottom_end + top_offset, bottom_start, bottom_end],
                [
                    Vector2::new(texture_start, scroll),
                    Vector2::new(texture_end, scroll),
                    Vector2::new(texture_start, 1.0 + scroll),
                    Vector2::new(texture_end, 1.0 + scroll),
                ],
            );
        }
    }

    fn render_ground_glow(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        anchor: Point3<f32>,
        screen_size: ScreenSize,
        time: f32,
    ) {
        let pulse = 0.65 + 0.35 * (time * 2.0).sin();
        let color = Color::rgba(0.22 * pulse, 0.42 * pulse, 0.85 * pulse, 1.0);

        // Order the corners relative to the camera so that the quad doesn't get
        // flipped when the player rotates the view.
        let right_vector = camera.look_up_vector().cross(camera.view_direction()).normalize();
        let mut forward_vector = Vector3::new(camera.view_direction().x, 0.0, camera.view_direction().z);

        if forward_vector.magnitude2() < f32::EPSILON {
            forward_vector = Vector3::new(0.0, 0.0, 1.0);
        }

        let forward_vector = forward_vector.normalize();
        let right_vector = Vector3::new(right_vector.x, 0.0, right_vector.z).normalize();
        let height = Vector3::new(0.0, GROUND_GLOW_HEIGHT, 0.0);

        let north_west = anchor + height + forward_vector * GROUND_GLOW_RADIUS - right_vector * GROUND_GLOW_RADIUS;
        let north_east = anchor + height + forward_vector * GROUND_GLOW_RADIUS + right_vector * GROUND_GLOW_RADIUS;
        let south_west = anchor + height - forward_vector * GROUND_GLOW_RADIUS - right_vector * GROUND_GLOW_RADIUS;
        let south_east = anchor + height - forward_vector * GROUND_GLOW_RADIUS + right_vector * GROUND_GLOW_RADIUS;

        render_world_quad(
            renderer,
            camera,
            anchor,
            screen_size,
            self.particle_texture.clone(),
            color,
            [north_west, north_east, south_west, south_east],
            [
                Vector2::new(0.0, 1.0),
                Vector2::new(1.0, 1.0),
                Vector2::new(0.0, 0.0),
                Vector2::new(1.0, 0.0),
            ],
        );
    }

    fn render_particles(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        anchor: Point3<f32>,
        screen_size: ScreenSize,
        time: f32,
    ) {
        let right_vector = camera.look_up_vector().cross(camera.view_direction()).normalize();
        let up_vector = camera.view_direction().cross(right_vector).normalize();

        for index in 0..PARTICLE_COUNT {
            // Distribute the particles evenly so that they don't all rise together.
            let seed = index as f32 / PARTICLE_COUNT as f32;
            let phase = (time * 0.4 + seed) % 1.0;
            let angle = seed * TAU * 2.0 + time * 0.25;
            let radius = 2.5 + 4.0 * (seed * 13.37).fract();

            let particle_position = anchor + Vector3::new(angle.cos() * radius, phase * PARTICLE_HEIGHT, angle.sin() * radius);
            let size = PARTICLE_MIN_SIZE + PARTICLE_SIZE_VARIATION * (seed * 5.0).fract();
            // Fade the sparkles in while they rise and out when they reach the top.
            let fade = (phase * PI).sin();

            if fade <= 0.0 {
                continue;
            }

            let color = Color::rgba(0.75 * fade, 0.85 * fade, fade, 1.0);
            let horizontal = right_vector * size;
            let vertical = up_vector * size;

            render_world_quad(
                renderer,
                camera,
                anchor,
                screen_size,
                self.particle_texture.clone(),
                color,
                [
                    particle_position - horizontal + vertical,
                    particle_position + horizontal + vertical,
                    particle_position - horizontal - vertical,
                    particle_position + horizontal - vertical,
                ],
                [
                    Vector2::new(0.0, 0.0),
                    Vector2::new(1.0, 0.0),
                    Vector2::new(0.0, 1.0),
                    Vector2::new(1.0, 1.0),
                ],
            );
        }
    }
}

/// Rotates the four corners of a ground aligned quad around the anchor (only
/// the x/z components, so it stays on the ground).
fn rotate_ground_corners(corners: [Point3<f32>; 4], anchor: Point3<f32>, angle: f32) -> [Point3<f32>; 4] {
    corners.map(|corner| {
        let offset = corner - anchor;
        let x = offset.x * angle.cos() - offset.z * angle.sin();
        let z = offset.x * angle.sin() + offset.z * angle.cos();

        anchor + Vector3::new(x, offset.y, z)
    })
}

#[cfg(test)]
mod tests {
    use cgmath::{InnerSpace, Point3, Vector2};

    use super::rotate_ground_corners;
    use crate::world::effect::order_quad;

    fn quad() -> [Vector2<f32>; 4] {
        [
            Vector2::new(0.0, 0.0),
            Vector2::new(2.0, 0.0),
            Vector2::new(0.0, 2.0),
            Vector2::new(2.0, 2.0),
        ]
    }

    #[test]
    fn quad_corners_are_ordered_top_left_top_right_bottom_left_bottom_right() {
        let (corners, ..) = order_quad(quad(), quad(), [1.0; 4]);

        assert_eq!(corners[0], Vector2::new(0.0, 0.0));
        assert_eq!(corners[1], Vector2::new(2.0, 0.0));
        assert_eq!(corners[2], Vector2::new(0.0, 2.0));
        assert_eq!(corners[3], Vector2::new(2.0, 2.0));
    }

    #[test]
    fn ground_rotation_keeps_the_quad_on_the_ground() {
        let anchor = Point3::new(10.0, 0.0, 10.0);
        let corners = [
            Point3::new(9.0, 0.2, 9.0),
            Point3::new(11.0, 0.2, 9.0),
            Point3::new(9.0, 0.2, 11.0),
            Point3::new(11.0, 0.2, 11.0),
        ];

        let rotated = rotate_ground_corners(corners, anchor, std::f32::consts::FRAC_PI_2);

        for corner in rotated {
            assert!((corner.y - 0.2).abs() < 1e-5);
            assert!((corner - anchor).magnitude() > 0.5);
        }
    }
}
