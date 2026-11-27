//! `CRagEffect::Hit1()` / `Hit2(int option)` - `RagEffect.cpp:730-859`.
//!
//! `Hit1` is `EF_HIT1` (the standard auto-attack impact): one blue
//! `PP_3DCYLINDER` ring and four `particle1` sparks (two `PP_3DPARTICLE`, two
//! `PP_3DPARTICLEGRAVITY`).
//!
//! `Hit2` is the generic eight-fragment impact (`EF_HIT2`, and the hit effect
//! of most physical skills via `Hit2(0)`/`Hit2(1)`). Eight `PP_2DTEXTURE`
//! shards are fired on a 45-degree ring with alternating
//! `lens1.tga`/`lens2.tga`.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `g_depthAverageHalf` - `MapSkyMgr.cpp:27`.
const DEPTH_AVERAGE_HALF: f32 = 0.5;

/// `CRagEffect::Hit1()` - `RagEffect.cpp:730-799`; lifetime 50
/// (`EffectTypes.cpp:9`).
///
/// The official body never plays a sound; the impact wave comes from the
/// damage message (`GameObject.cpp:1780-1870`).
#[derive(Debug, Default)]
pub struct Hit1;

impl PortedEffect for Hit1 {
    fn lifetime(&self) -> i32 {
        50
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();

        // `int facing = (int)(180.0f - state.parent->render.angle);`
        let mut facing = (180.0f32 - runtime.parent_angle) as i32;
        if facing < 0 {
            facing += 360;
        }
        let facing_degrees = facing as f32;

        // One `PP_3DCYLINDER` ring: `effect\ring_blue.tga`, 10 frames.
        let ring = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);
        state.duration = 10;
        state.longitude = facing_degrees;
        state.latitude = -90.0;
        state.transform.make_x_rotation(angles.radian(-90));
        state.transform.append_y_rotation(angles.radian(facing));
        state.delta_pos2 = [0.0, -10.0, 0.0];
        state.speed = 0.7;
        state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
        state.outer_size = 10.0;
        state.inner_size = 5.0;
        state.height_size = 3.5;
        state.fade_out_cnt = state.duration - state.duration / 2;
        state.texture = Some("effect\\ring_blue.tga");

        // Two `PP_3DPARTICLE` sparks. RNG order per particle: longitude (80),
        // latitude (100), travel (4), speed (90), duration (24), size (100).
        for _ in 0..2 {
            let longitude = (runtime.rng.clamped_random(80) + (facing - 40)) as f32;
            let latitude = (runtime.rng.clamped_random(100) - 50) as f32;
            let travel = (runtime.rng.clamped_random(4) + 4) as f32;
            let speed = (runtime.rng.clamped_random(90) + 60) as f32 / 100.0;
            let duration = runtime.rng.clamped_random(24) + 6;
            let size = (runtime.rng.clamped_random(100) + 60) as f32 / 100.0;

            let spark = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spark);

            state.longitude = longitude;
            state.latitude = latitude;
            state.transform.make_y_rotation(angles.radian(longitude as i32));
            state.transform.append_x_rotation(angles.radian(latitude as i32));

            let placed = matrix_mult([0.0, 0.0, travel], &state.transform.values);
            state.delta_pos3 = placed;
            state.delta_pos2 = [placed[0] + 0.0, placed[1] - 10.0, placed[2] + 0.0];

            state.speed = speed;
            state.duration = duration;
            state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
            // `Hit1` does not set `sizeSpeed` (unlike `Hit3`).
            state.size = size;
            state.fade_out_cnt = 0;
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.num_segment = 3;
            state.init_segment();
        }

        // Two `PP_3DPARTICLEGRAVITY` sparks. RNG order per particle: longitude
        // (80), latitude (100), travel (4), duration (24), speed (90),
        // gravSpeed (90), size (100).
        for _ in 0..2 {
            let longitude = (runtime.rng.clamped_random(80) + (facing + 140)) as f32;
            let latitude = (runtime.rng.clamped_random(100) - 50) as f32;
            let travel = (runtime.rng.clamped_random(4) + 4) as f32;
            let duration = runtime.rng.clamped_random(24) + 6;
            let speed = (runtime.rng.clamped_random(90) + 60) as f32 / 100.0;
            let grav_speed = -((runtime.rng.clamped_random(90) + 30) as f32) / 100.0;
            let size = (runtime.rng.clamped_random(100) + 60) as f32 / 100.0;

            let spark = runtime.launch(PrimitiveId::Particle3dGravity, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spark);

            state.longitude = longitude;
            state.latitude = latitude;
            state.transform.make_y_rotation(angles.radian(longitude as i32));
            state.transform.append_x_rotation(angles.radian(latitude as i32));

            let placed = matrix_mult([0.0, 0.0, travel], &state.transform.values);
            state.delta_pos3 = placed;
            state.delta_pos2 = [placed[0] + 0.0, placed[1] - 10.0, placed[2] + 0.0];
            // The original re-applies the Y rotation before storing the
            // duration (the two `MakeYRotation` calls in `RagEffect.cpp:792`).
            state.transform.make_y_rotation(angles.radian(longitude as i32));

            state.duration = duration;
            state.speed = speed;
            state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
            state.grav_speed = grav_speed;
            state.grav_accel = -(state.grav_speed / state.duration as f32 * 2.0);
            state.size = size;
            state.fade_out_cnt = 0;
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.num_segment = 3;
            state.init_segment();
        }

        true
    }
}

#[derive(Debug)]
pub struct Hit2 {
    option: i32,
}

impl Hit2 {
    pub fn new(option: i32) -> Self {
        Self { option }
    }
}

impl Default for Hit2 {
    fn default() -> Self {
        Self { option: 0 }
    }
}

impl PortedEffect for Hit2 {
    fn lifetime(&self) -> i32 {
        50
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        runtime.play_sound("effect\\EF_hit2.wav");

        let angles = Angles::shared();

        let mut index = 0usize;
        let mut degree = 0.0f32;

        while degree < 360.0 {
            // RNG order: duration, heading offset, speed, width, height, spread.
            let duration = runtime.rng.clamped_random(20) + 10;
            let heading = (degree - 15.0) + runtime.rng.clamped_random(30) as f32;
            let speed = (runtime.rng.clamped_random(45) + 5) as f32 / 10.0;
            let width_size = (runtime.rng.clamped_random(15) + 5) as f32;
            let height_size = (runtime.rng.clamped_random(20) + 20) as f32;
            let spread = runtime.rng.clamped_random(5) as f32;

            let shard = runtime.launch(PrimitiveId::Texture2d, [0.0, 0.0, 0.0]);
            {
                let state = runtime.primitive_mut(shard);
                state.pattern = 1;
                state.duration = duration;
                state.roll = heading;
                state.longitude = heading;
                state.speed = speed;
                state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
                state.width_size = width_size;
                state.height_size = height_size;
                state.width_speed = -(state.width_size / state.duration as f32);
                state.height_speed = 1.5;
                state.height_accel = 0.25;

                state.delta_pos2[0] = angles.sine(degree as i32) * spread;
                state.delta_pos2[1] = angles.cosine(degree as i32) * -spread;
                state.delta_pos2[1] -= 20.0;

                if self.option != 0 {
                    state.red = 120;
                    state.green = 120;
                }

                state.alpha = 0.0;
                state.alpha_speed = state.max_alpha * 0.125;
                state.texture = Some(if index % 2 == 0 { "effect\\lens1.tga" } else { "effect\\lens2.tga" });
            }

            // The original alternates the texture by fragment index; the
            // texture assignment happens in the renderer.
            index += 1;
            let _ = index;

            degree += 45.0;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn spawns_eight_fragments() {
        let mut effect = Hit2::default();
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 51);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_hit2.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 8);

        for (index, fragment) in runtime.primitives.iter().enumerate() {
            let state = &fragment.state;
            assert_eq!(state.count, PrimitiveId::Texture2d);
            assert_eq!(state.pattern, 1);
            assert!((10..=29).contains(&state.duration), "duration {}", state.duration);
            assert!((0.5..=4.9).contains(&state.speed));
            assert!((5.0..=19.0).contains(&state.width_size));
            assert!((20.0..=39.0).contains(&state.height_size));
            // Spawned at heightSpeed 1.5 / heightAccel 0.25, one step applied.
            assert!((state.height_speed - 1.75).abs() < 1e-5);
            assert!((state.height_accel - 0.25).abs() < 1e-6);
            // Heading is degree - 15 + random(30).
            let expected_center = index as f32 * 45.0;
            assert!((state.longitude - expected_center).abs() <= 15.0);
        }
    }

    #[test]
    fn tinted_option_sets_rgb_to_120() {
        let mut effect = Hit2::new(1);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 51);

        step(&mut effect, &mut runtime);

        for fragment in &runtime.primitives {
            assert_eq!(fragment.state.red, 120);
            assert_eq!(fragment.state.green, 120);
        }
    }

    // -- Hit1 ---------------------------------------------------------------

    #[test]
    fn hit1_spawns_one_ring_and_four_sparks_with_the_official_values() {
        let mut effect = Hit1;
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 7);
        runtime.parent_angle = 0.0; // facing = 180

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.sounds, Vec::<String>::new(), "Hit1 plays no sound");
        assert_eq!(runtime.primitives.len(), 5);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::Cylinder3d);
        assert_eq!(ring.duration, 10);
        assert_eq!(ring.texture, Some("effect\\ring_blue.tga"));
        assert_eq!(ring.longitude, 180.0);
        assert_eq!(ring.latitude, -90.0);
        assert_eq!(ring.delta_pos2, [0.0, -10.0, 0.0]);
        assert!((ring.speed - 0.7).abs() < 1e-6);
        assert!((ring.accel - -(0.7 / 10.0) * DEPTH_AVERAGE_HALF).abs() < 1e-6);
        assert_eq!(ring.outer_size, 10.0);
        assert_eq!(ring.inner_size, 5.0);
        assert_eq!(ring.height_size, 3.5);
        assert_eq!(ring.fade_out_cnt, 5);

        let mut particles = 0;
        let mut gravity = 0;
        for primitive in &runtime.primitives[1..] {
            let state = &primitive.state;
            if state.count == PrimitiveId::Particle3d {
                particles += 1;
            } else if state.count == PrimitiveId::Particle3dGravity {
                gravity += 1;
            }
            assert_eq!(state.sprite, Some("이팩트\\particle1.spr"));
            assert_eq!(state.action, Some("이팩트\\particle1.act"));
            assert_eq!(state.num_segment, 3);
            assert!((6..=29).contains(&state.duration), "duration {}", state.duration);
            assert!((0.6..=1.59).contains(&state.size), "size {}", state.size);
            assert!((state.delta_pos2[1] - (state.delta_pos3[1] - 10.0)).abs() < 1e-6);
        }

        assert_eq!(particles, 2);
        assert_eq!(gravity, 2);
    }

    #[test]
    fn hit1_gravity_particles_fall() {
        let mut effect = Hit1;
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 9);

        assert!(effect.on_process(&mut runtime));

        for primitive in &runtime.primitives {
            let state = &primitive.state;
            if state.count != PrimitiveId::Particle3dGravity {
                continue;
            }
            assert!((-1.2..=-0.3).contains(&state.grav_speed), "gravSpeed {}", state.grav_speed);
            assert!((state.grav_accel - -(state.grav_speed / state.duration as f32 * 2.0)).abs() < 1e-6);
        }
    }

    #[test]
    fn hit1_uses_the_parent_facing() {
        for (parent_angle, expected) in [(0.0, 180), (90.0, 90), (270.0, -90 + 360)] {
            let mut effect = Hit1;
            let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 3);
            runtime.parent_angle = parent_angle;

            assert!(effect.on_process(&mut runtime));

            assert_eq!(runtime.primitives[0].state.longitude, expected as f32, "parent {parent_angle}");
        }
    }
}
