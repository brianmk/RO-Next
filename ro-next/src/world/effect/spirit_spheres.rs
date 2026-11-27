//! Monk/Champion spirit spheres ("Dangerous Soul Collect"/Zen and the normal
//! spirit-sphere count) drawn around the character, matching the classic
//! client: blue-white glowing balls that orbit the affected entity instead of
//! a small HUD counter.
//!
//! In the official client the orbs are **not** an actor aura: `ZC_SPIRITS2`
//! launches the hard-coded effect `EF_CHOOKGI` (228) with the sphere count in
//! `AM_SETPOS` (`GameModePackets.cpp:4822-4893`). `CRagEffect::OnProcess`
//! dispatches it to `ElementalOrbit(0)` (`RagEffect.cpp:24415-24416`), which
//! arms one primitive 32 per orb (`ElementalOrbit`, `RagEffect.cpp:13519`);
//! `CEffectPrim::PrimDriftingOrbs` (`EffectPrimitiveMotion.cpp:7461`) then
//! orbits them. This port keeps the visual in a client-side renderer instead
//! of porting primitive 32: the update loop only receives seconds, not the
//! effect runtime, so the exact orbit values below are copied from
//! `PrimDriftingOrbs`/`ElementalOrbit`.
//!
//! **USER TWEAK:** the rendered orbit radius is 5 % wider than the decompiled
//! `PrimDriftingOrbs` constant (7.0 -> 7.35) by explicit user request. That is
//! a display tweak, not a fidelity value; a future fidelity pass should drop
//! [`ORBIT_RADIUS_USER_SCALE`] and render 7.0 again.
//!
//! The official client uses `data\texture\effect\thunder_center.bmp` for the
//! spheres. Each sphere is a stack of additive, camera-facing quads with
//! decreasing size and increasing alpha; the inner white core gives the blue
//! ball its bright centre. The animation is purely a function of the client
//! tick, so no allocations happen per frame.

use std::f32::consts::TAU;
use std::sync::Arc;

use cgmath::{InnerSpace, Point3, Vector2, Vector3};
use wgpu::BlendFactor;

use super::render_world_quad_blended;
use crate::graphics::{Color, ScreenSize, Texture};
use crate::renderer::EffectRenderer;
use crate::world::Camera;

/// Official radius of the orbit around the character in world units.
///
/// `CEffectPrim::PrimDriftingOrbs` adds
/// `g_sinTable[height[6]] * 7.0` to the master position
/// (`EffectPrimitiveMotion.cpp:7539-7540`); `ElementalOrbit(0)` seeds
/// `height[6] = index * 72` (`RagEffect.cpp:13573`).
const ORBIT_RADIUS: f32 = 7.0;
/// USER TWEAK: explicit user request - render the orbit 5 % wider than the
/// decompiled radius (7.0 -> 7.35). This is **not** a fidelity value; a future
/// fidelity pass should drop this factor (make it `1.0`) and keep
/// [`ORBIT_RADIUS`] at 7.0.
const ORBIT_RADIUS_USER_SCALE: f32 = 1.05;
/// Radius the port actually renders: `7.0 * 1.05 = 7.35` (see
/// [`ORBIT_RADIUS_USER_SCALE`]).
const RENDERED_ORBIT_RADIUS: f32 = ORBIT_RADIUS * ORBIT_RADIUS_USER_SCALE;
/// Height of the orbit above the character's feet.
///
/// The client effect space is y-down, so
/// `slots[0].height[7] = -15.0F` (`RagEffect.cpp:13574`) is 15 world units
/// above the actor's feet.
const ORBIT_HEIGHT: f32 = 15.0;
/// Rotation speed of the orbit in radians per second.
///
/// `PrimDriftingOrbs` advances `height[6]` by one degree per logic tick
/// (`EffectPrimitiveMotion.cpp:7515`) and the client's logic timer runs at
/// 60 Hz (`InitTimer(60)`, `Launcher.cpp:1522`), i.e. 60 degrees per second.
const ORBIT_SPEED: f32 = TAU / 6.0;
/// Angle of the first sphere in radians: `angle` starts at zero and every
/// following sphere adds 72 degrees (`RagEffect.cpp:13535`, `:13573`).
const START_ANGLE: f32 = 0.0;

/// Number of spheres the classic spirit-sphere system supports.
fn sphere_count(amount: i16) -> usize {
    amount.clamp(0, 5) as usize
}

/// Centre of sphere `index` after `time` seconds, in world units.
///
/// The official client uses `x = sin(angle) * radius` and
/// `z = cos(angle) * radius` (`EffectPrimitiveMotion.cpp:7539-7540`), so the
/// first sphere starts on the positive z axis and the orbit advances
/// 60 degrees per second.
fn sphere_center(position: Point3<f32>, index: usize, time: f32) -> Point3<f32> {
    let angle = START_ANGLE + index as f32 * (TAU / 5.0) + time * ORBIT_SPEED;

    // USER TWEAK: this is the single sphere-orbit radius; it is 7.0 * 1.05 =
    // 7.35 instead of the decompiled 7.0 (explicit user request). No other
    // effect shares it.
    let radius = RENDERED_ORBIT_RADIUS;

    position + Vector3::new(radius * angle.sin(), ORBIT_HEIGHT, radius * angle.cos())
}

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
    let count = sphere_count(amount);
    if count == 0 {
        return;
    }

    let right = camera.look_up_vector().cross(camera.view_direction()).normalize();
    let up = camera.view_direction().cross(right).normalize();

    for index in 0..count {
        let center = sphere_center(position, index, time);

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

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-4;

    fn assert_close(actual: f32, expected: f32) {
        assert!((actual - expected).abs() < EPSILON, "expected {expected}, got {actual}");
    }

    #[test]
    fn orbit_uses_the_official_prim_drifting_orbs_radius_and_lift() {
        // `EffectPrimitiveMotion.cpp:7539-7540` (radius) and
        // `RagEffect.cpp:13574` + the client's y-down effect space (lift).
        assert_eq!(ORBIT_RADIUS, 7.0);
        assert_eq!(ORBIT_HEIGHT, 15.0);
    }

    #[test]
    fn the_rendered_radius_is_the_official_value_times_the_user_tweak() {
        // Explicit user request: 5 % wider than the decompiled radius.
        assert!((RENDERED_ORBIT_RADIUS - 7.35).abs() < 1e-3, "official 7.0 * 1.05 user tweak");
        assert_eq!(ORBIT_RADIUS, 7.0);
        assert_eq!(ORBIT_RADIUS_USER_SCALE, 1.05);
    }

    #[test]
    fn orbit_advances_sixty_degrees_per_second() {
        // One degree per 60 Hz logic tick (`EffectPrimitiveMotion.cpp:7515`,
        // `Launcher.cpp:1522`): 60 degrees = TAU / 6 radians per second.
        assert_close(ORBIT_SPEED, TAU / 6.0);
        assert_eq!(START_ANGLE, 0.0);
    }

    #[test]
    fn first_sphere_starts_on_the_positive_z_axis() {
        // `x = sin(0) * 7.35 = 0`, `z = cos(0) * 7.35 = 7.35`, 15 units above
        // the feet.
        let center = sphere_center(Point3::new(10.0, 2.0, 20.0), 0, 0.0);

        assert_close(center.x, 10.0);
        assert_close(center.y, 17.0);
        assert_close(center.z, 20.0 + RENDERED_ORBIT_RADIUS);
    }

    #[test]
    fn five_spheres_are_seventy_two_degrees_apart() {
        let origin = Point3::new(0.0, 0.0, 0.0);
        let second = sphere_center(origin, 1, 0.0);
        let fifth = sphere_center(origin, 4, 0.0);

        assert_close(second.x, RENDERED_ORBIT_RADIUS * 72.0_f32.to_radians().sin());
        assert_close(second.z, RENDERED_ORBIT_RADIUS * 72.0_f32.to_radians().cos());
        assert_close(fifth.x, RENDERED_ORBIT_RADIUS * 288.0_f32.to_radians().sin());
        assert_close(fifth.z, RENDERED_ORBIT_RADIUS * 288.0_f32.to_radians().cos());
    }

    #[test]
    fn the_orbit_moves_counter_clockwise_from_positive_z() {
        let origin = Point3::new(0.0, 0.0, 0.0);
        let after_one_second = sphere_center(origin, 0, 1.0);

        // 60 degrees per second: `x = sin(60) * 7.35`, `z = cos(60) * 7.35`.
        assert_close(after_one_second.x, RENDERED_ORBIT_RADIUS * 60.0_f32.to_radians().sin());
        assert_close(after_one_second.z, RENDERED_ORBIT_RADIUS * 60.0_f32.to_radians().cos());
    }

    #[test]
    fn the_five_sphere_slots_wrap_onto_the_first_sphere() {
        let origin = Point3::new(0.0, 0.0, 0.0);
        let first = sphere_center(origin, 0, 0.25);
        let sixth = sphere_center(origin, 5, 0.25);

        assert_close(first.x, sixth.x);
        assert_close(first.z, sixth.z);
    }

    #[test]
    fn sphere_count_is_clamped_to_five() {
        assert_eq!(sphere_count(0), 0);
        assert_eq!(sphere_count(3), 3);
        assert_eq!(sphere_count(5), 5);
        assert_eq!(sphere_count(9), 5);
        assert_eq!(sphere_count(-2), 0);
    }
}
