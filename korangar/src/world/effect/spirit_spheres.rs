//! Monk/Champion spirit spheres ("Dangerous Soul Collect"/Zen and the normal
//! spirit-sphere count) drawn around the character, matching the classic
//! client (and roBrowserLegacy's `SpiritSphere` renderer): blue-white glowing
//! balls that orbit the affected entity instead of a small HUD counter.
//!
//! The official client uses `data\texture\effect\thunder_center.bmp` for the
//! spheres (roBrowserLegacy loads the same file). Each sphere is a stack of
//! additive, camera-facing quads with decreasing size and increasing alpha;
//! the inner white core gives the blue ball its bright centre. The animation
//! is purely a function of the client tick, so no allocations happen per
//! frame.

use std::f32::consts::TAU;
use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Vector2, Vector3};
use wgpu::BlendFactor;

use super::render_world_quad_blended;
use crate::graphics::{Color, ScreenSize, Texture};
use crate::renderer::EffectRenderer;
use crate::world::Camera;

/// Radius of the orbit around the character in world units.
const ORBIT_RADIUS: f32 = 3.52;
/// Height of the orbit above the character's feet.
const ORBIT_HEIGHT: f32 = 20.0;
/// Rotation speed of the orbit in radians per second.
const ORBIT_SPEED: f32 = 0.9;
/// Angular spacing between the spheres (in radians) when the count changes.
const START_ANGLE: f32 = -0.4;

/// Draws `amount` orbiting spirit spheres around `position`.
///
/// `amount` is clamped to the five spheres of the classic spirit-sphere
/// system; the state can temporarily report more while a server packet is
/// being applied, so rendering more than five would not match the game.
pub fn render(
    renderer: &mut EffectRenderer,
    camera: &dyn Camera,
    texture: Arc<Texture>,
    position: Point3<f32>,
    screen_size: ScreenSize,
    amount: i16,
    time: f32,
) {
    let count = amount.clamp(0, 5) as usize;
    if count == 0 {
        return;
    }

    let right = camera.look_up_vector().cross(camera.view_direction()).normalize();
    let up = camera.view_direction().cross(right).normalize();
    let anchor = position + Vector3::new(0.0, ORBIT_HEIGHT, 0.0);
    let step = TAU / 5.0;

    for index in 0..count {
        let angle = START_ANGLE + index as f32 * step + time * ORBIT_SPEED;
        let center = anchor + Vector3::new(angle.cos() * ORBIT_RADIUS, 0.0, angle.sin() * ORBIT_RADIUS);

        // Outer blue halo, mid blue body, white core: additive blending makes
        // the overlapping quads read as one glowing sphere.
        render_ball(
            renderer,
            camera,
            screen_size,
            texture.clone(),
            center,
            right,
            up,
            2.7,
            Color::rgba(0.05, 0.18, 0.65, 0.20),
        );
        render_ball(
            renderer,
            camera,
            screen_size,
            texture.clone(),
            center,
            right,
            up,
            1.8,
            Color::rgba(0.10, 0.35, 0.95, 0.55),
        );
        render_ball(
            renderer,
            camera,
            screen_size,
            texture.clone(),
            center,
            right,
            up,
            1.1,
            Color::rgba(0.55, 0.70, 1.0, 0.95),
        );
        render_ball(
            renderer,
            camera,
            screen_size,
            texture.clone(),
            center,
            right,
            up,
            0.55,
            Color::rgba(0.95, 0.97, 1.0, 1.0),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_ball(
    renderer: &mut EffectRenderer,
    camera: &dyn Camera,
    screen_size: ScreenSize,
    texture: Arc<Texture>,
    center: Point3<f32>,
    right: Vector3<f32>,
    up: Vector3<f32>,
    size: f32,
    color: Color,
) {
    let horizontal = right * size;
    let vertical = up * size;
    let corners = [
        center - horizontal - vertical,
        center + horizontal - vertical,
        center - horizontal + vertical,
        center + horizontal + vertical,
    ];

    render_world_quad_blended(
        renderer,
        camera,
        center,
        screen_size,
        texture,
        color,
        corners,
        [
            Vector2::new(0.0, 0.0),
            Vector2::new(1.0, 0.0),
            Vector2::new(0.0, 1.0),
            Vector2::new(1.0, 1.0),
        ],
        BlendFactor::One,
        BlendFactor::One,
    );
}
