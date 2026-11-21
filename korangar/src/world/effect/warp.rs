use std::f32::consts::{PI, TAU};
use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Rad, Vector2, Vector3};
use wgpu::BlendFactor;

use crate::graphics::{Color, ScreenSize, Texture};
use crate::loaders::{ImageType, SpriteLoader, TextureLoader};
use crate::renderer::EffectRenderer;
use crate::world::Camera;

/// The origin that the original client uses for hard-coded effects. Local
/// effect coordinates are expressed relative to the feet of the effect anchor
/// on a 640x480 screen, which is exactly the space that [`EffectRenderer`]
/// expects.
const EFFECT_ORIGIN: Vector2<f32> = Vector2::new(319.0, 291.0);

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

/// Textures used by the warp portal effect. They are loaded once when the
/// client starts, because they are shared between all warps on the map.
pub struct WarpPortal {
    streak_texture: Arc<Texture>,
    gradient_texture: Arc<Texture>,
    particle_texture: Arc<Texture>,
}

impl WarpPortal {
    /// Returns `None` if the game files don't contain the effect textures, in
    /// which case warps simply stay invisible.
    pub fn new(texture_loader: &TextureLoader, sprite_loader: &SpriteLoader) -> Option<Self> {
        let streak_texture = texture_loader.get_or_load("effect\\ring_blue.tga", ImageType::Color).ok()?;
        let gradient_texture = texture_loader.get_or_load("effect\\alpha_down.tga", ImageType::Color).ok()?;
        let particle_sprite = sprite_loader.get_or_load("이팩트\\particle1.spr").ok()?;
        let particle_texture = particle_sprite.textures.first()?.clone();

        Some(Self {
            streak_texture,
            gradient_texture,
            particle_texture,
        })
    }

    pub fn render(&self, renderer: &mut EffectRenderer, camera: &dyn Camera, position: Point3<f32>, screen_size: ScreenSize, time: f32) {
        if (camera.focus_point() - position).magnitude() > MAXIMUM_DISTANCE {
            return;
        }

        let anchor = position;
        let Some((anchor_screen, _anchor_depth)) = Self::project(camera, anchor, screen_size) else {
            return;
        };

        let rotation = time * 0.15;
        let scroll = (time * 0.55) % 1.0;

        self.render_cylinder(
            renderer,
            camera,
            anchor,
            anchor_screen,
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
            anchor_screen,
            screen_size,
            self.gradient_texture.clone(),
            Color::rgba(0.12, 0.24, 0.46, 1.0),
            rotation,
            0.0,
        );

        self.render_ground_glow(renderer, camera, anchor, anchor_screen, screen_size, time);
        self.render_particles(renderer, camera, anchor, anchor_screen, screen_size, time);
    }

    #[allow(clippy::too_many_arguments)]
    fn render_cylinder(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        anchor: Point3<f32>,
        anchor_screen: Vector2<f32>,
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

            self.render_quad(
                renderer,
                camera,
                anchor,
                anchor_screen,
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
        anchor_screen: Vector2<f32>,
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

        self.render_quad(
            renderer,
            camera,
            anchor,
            anchor_screen,
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
        anchor_screen: Vector2<f32>,
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

            self.render_quad(
                renderer,
                camera,
                anchor,
                anchor_screen,
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

    /// Renders a single textured quad. The corners are given in world space in
    /// counter-clockwise order starting at the top left corner, with the
    /// matching texture coordinates. They are projected and re-ordered so
    /// that the quad is never rendered flipped, no matter how the camera is
    /// rotated.
    #[allow(clippy::too_many_arguments)]
    fn render_quad(
        &self,
        renderer: &mut EffectRenderer,
        camera: &dyn Camera,
        anchor: Point3<f32>,
        anchor_screen: Vector2<f32>,
        screen_size: ScreenSize,
        texture: Arc<Texture>,
        color: Color,
        corners: [Point3<f32>; 4],
        texture_coordinates: [Vector2<f32>; 4],
    ) {
        let mut screen_corners = [Vector2::new(0.0, 0.0); 4];
        let mut corner_depths = [0.0; 4];

        for (index, corner) in corners.iter().enumerate() {
            let Some((projected, depth)) = Self::project(camera, *corner, screen_size) else {
                return;
            };

            screen_corners[index] = projected - anchor_screen;
            corner_depths[index] = depth;
        }

        let (screen_corners, texture_coordinates, corner_depths) = order_quad(screen_corners, texture_coordinates, corner_depths);

        renderer.render_effect_with_depth(
            camera,
            anchor,
            texture,
            screen_corners.map(|corner| EFFECT_ORIGIN + corner),
            corner_depths,
            [
                texture_coordinates[3],
                texture_coordinates[1],
                texture_coordinates[0],
                texture_coordinates[2],
            ],
            Vector2::new(0.0, 0.0),
            Rad(0.0),
            color,
            BlendFactor::One,
            BlendFactor::One,
        );
    }

    /// Projects a world position onto the screen, returning `None` if the
    /// position is behind the camera. The result is in physical window
    /// pixels with the origin in the top left corner, which is the space
    /// that [`EffectRenderer::render_effect`] uses.
    fn project(camera: &dyn Camera, position: Point3<f32>, screen_size: ScreenSize) -> Option<(Vector2<f32>, f32)> {
        let clip_space_position = camera.view_projection_matrix() * position.to_homogeneous();

        if clip_space_position.w <= 0.0 {
            return None;
        }

        let normalized_x = clip_space_position.x / clip_space_position.w;
        let normalized_y = clip_space_position.y / clip_space_position.w;
        let depth = clip_space_position.z / clip_space_position.w;

        Some((
            Vector2::new(
                (normalized_x * 0.5 + 0.5) * screen_size.width,
                (0.5 - normalized_y * 0.5) * screen_size.height,
            ),
            depth,
        ))
    }
}

/// Re-orders the corners of a screen space quad (together with their texture
/// coordinates) into top left, top right, bottom left, bottom right. This makes
/// sure that a quad that is seen from "behind" doesn't render mirrored or
/// degenerate, which would happen if the projected corners were handed to the
/// renderer in an order that is no longer a valid polygon.
fn order_quad(
    corners: [Vector2<f32>; 4],
    texture_coordinates: [Vector2<f32>; 4],
    depths: [f32; 4],
) -> ([Vector2<f32>; 4], [Vector2<f32>; 4], [f32; 4]) {
    let mut indices = [0, 1, 2, 3];
    indices.sort_by(|&left, &right| corners[left].y.total_cmp(&corners[right].y));

    let mut top = [indices[0], indices[1]];
    top.sort_by(|&left, &right| corners[left].x.total_cmp(&corners[right].x));

    let mut bottom = [indices[2], indices[3]];
    bottom.sort_by(|&left, &right| corners[left].x.total_cmp(&corners[right].x));

    let order = [top[0], top[1], bottom[0], bottom[1]];

    (
        order.map(|index| corners[index]),
        order.map(|index| texture_coordinates[index]),
        order.map(|index| depths[index]),
    )
}

#[cfg(test)]
mod tests {
    use cgmath::Vector2;

    use super::order_quad;

    fn uv_corners() -> [Vector2<f32>; 4] {
        [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
        ]
    }

    /// The corner ordering has to produce a valid (non self-intersecting)
    /// polygon for both a quad seen from the front and one seen from the
    /// back.
    #[test]
    fn orders_quads_consistently() {
        let front = [
            Vector2::new(10.0, 10.0),
            Vector2::new(30.0, 10.0),
            Vector2::new(10.0, 50.0),
            Vector2::new(30.0, 50.0),
        ];
        let depths = [1.0, 2.0, 3.0, 4.0];
        let (corners, coordinates, ordered_depths) = order_quad(front, uv_corners(), depths);
        assert_eq!(corners[0], Vector2::new(10.0, 10.0));
        assert_eq!(corners[1], Vector2::new(30.0, 10.0));
        assert_eq!(corners[2], Vector2::new(10.0, 50.0));
        assert_eq!(corners[3], Vector2::new(30.0, 50.0));
        assert_eq!(coordinates[0], Vector2::new(0.0, 0.0));
        // The depths follow their corners.
        assert_eq!(ordered_depths, [1.0, 2.0, 3.0, 4.0]);

        // Same quad, but the two halves are swapped, which happens when a cylinder
        // segment is seen from the back.
        let back = [
            Vector2::new(30.0, 10.0),
            Vector2::new(10.0, 10.0),
            Vector2::new(30.0, 50.0),
            Vector2::new(10.0, 50.0),
        ];
        let (corners, coordinates, _depths) = order_quad(back, uv_corners(), depths);
        assert_eq!(corners[0], Vector2::new(10.0, 10.0));
        assert_eq!(corners[1], Vector2::new(30.0, 10.0));
        assert_eq!(corners[2], Vector2::new(10.0, 50.0));
        assert_eq!(corners[3], Vector2::new(30.0, 50.0));
        // The texture coordinates follow their corners.
        assert_eq!(coordinates[0], Vector2::new(1.0, 0.0));
        assert_eq!(coordinates[2], Vector2::new(1.0, 1.0));
    }

    /// A horizontal ground quad, which is what the ground glow uses.
    #[test]
    fn orders_ground_quads() {
        let corners = [
            Vector2::new(100.0, 300.0),
            Vector2::new(200.0, 300.0),
            Vector2::new(110.0, 200.0),
            Vector2::new(190.0, 200.0),
        ];
        let (ordered, ..) = order_quad(corners, uv_corners(), [0.0; 4]);
        assert_eq!(ordered[0], Vector2::new(110.0, 200.0));
        assert_eq!(ordered[1], Vector2::new(190.0, 200.0));
        assert_eq!(ordered[2], Vector2::new(100.0, 300.0));
        assert_eq!(ordered[3], Vector2::new(200.0, 300.0));
    }
}
