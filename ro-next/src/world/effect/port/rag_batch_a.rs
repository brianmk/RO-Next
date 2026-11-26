//! Effects ported by the "a" parallel batch.
//!
//! This file is owned exclusively by batch "a": bodies and their tests live
//! here so parallel work never edits a shared module.

#![allow(dead_code)]

use super::angles::Angles;
use super::math::{matrix_mult, Matrix};
use super::primitive::PrimitiveId;
use super::runtime::{EffectRuntime, PortedEffect};

/// `CRagEffect::Steal` - `RagEffect.cpp:1835-1868`; dispatch
/// `RagEffect.cpp:23187-23189`.
///
/// `EF_STEAL`: on frame 0 only, play `EF_Steal.wav` and launch ten primitive-6
/// sprite particles (`particle7.spr`) that fan out, arc down and shrink. The
/// whole per-particle setup (including the two `MakeYRotation` calls and the
/// `deltaPos3`/`deltaPos2` split) is reproduced line for line.
#[derive(Debug, Default)]
pub struct Steal {}

impl PortedEffect for Steal {
    fn lifetime(&self) -> i32 {
        50
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        runtime.play_sound("effect\\EF_Steal.wav");

        let angles = Angles::shared();
        let duration = runtime.lifetime;

        for _ in 0..10 {
            // ClampedRandom order: longitude, latitude, distance, speed,
            // gravity, size.
            let longitude = runtime.rng.clamped_random(360) as f32;
            let latitude = (runtime.rng.clamped_random(100) - 50) as f32;
            let distance = (runtime.rng.clamped_random(4) + 4) as f32;
            let speed = (runtime.rng.clamped_random(50) + 50) as f32 / 100.0;
            let grav_speed = -((runtime.rng.clamped_random(90) + 60) as f32) / 100.0;
            let size = (runtime.rng.clamped_random(100) + 100) as f32 / 100.0;

            let index = runtime.launch(PrimitiveId::Particle3dGravity, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(index);

            state.longitude = longitude;
            state.latitude = latitude;
            state.transform.make_y_rotation(angles.radian(longitude as i32));
            state.transform.append_x_rotation(angles.radian(latitude as i32));

            let forward = matrix_mult([0.0, 0.0, distance], &state.transform.values);
            state.delta_pos3 = forward;
            state.delta_pos2 = [forward[0] + 0.0, forward[1] - 10.0, forward[2] + 0.0];

            // The second MakeYRotation overwrites the combined rotation above.
            state.transform.make_y_rotation(angles.radian(longitude as i32));
            state.duration = duration;

            state.speed = speed;
            state.accel = -(speed / duration as f32) * 0.5;

            state.grav_speed = grav_speed;
            state.grav_accel = -(grav_speed / duration as f32) * 2.0;

            state.size = size;
            state.size_speed = -(size / duration as f32);

            state.fade_out_cnt = duration - duration / 3;

            state.sprite = Some("이팩트\\particle7.spr");
            state.action = Some("이팩트\\particle7.act");
            state.init_segment();
        }

        true
    }
}

/// `CRagEffect::TexturedForkBolt(name, flag)` - `RagEffect.cpp:7268-7321`.
///
/// On frames 0, 10 and 20 it launches one primitive-155 twin-glow-square
/// particle textured with `name`; `flag` is stored in `slots[0].flag1[0]`
/// (`0` for `EF_CHEMICAL_V`, `1` for `EF_CHEMICAL_V2`, `2` for the metallic
/// sound line). The body itself is reusable; the dispatch blocks that call it
/// also start a `TexturedBeam`, so no effect id is registered for it yet.
#[derive(Debug)]
pub struct TexturedForkBolt {
    name: &'static str,
    flag: i8,
    lifetime: i32,
}

impl TexturedForkBolt {
    pub fn new(name: &'static str, flag: i8, lifetime: i32) -> Self {
        Self { name, flag, lifetime }
    }
}

impl PortedEffect for TexturedForkBolt {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame >= 30 || frame % 10 != 0 {
            return true;
        }

        let duration = runtime.lifetime;
        let position = runtime.position;

        // `Get_Angle(deltaPos.x - render.position.x, deltaPos.z - render.position.z)`.
        // A zero vector falls through the table loop and stays at the
        // original's `180`, not `0` (see `EffectPrimitiveState.cpp:51-70`).
        let rise_angle = runtime.get_angle(
            runtime.target_offset[0] - position[0],
            runtime.target_offset[2] - position[2],
        );

        // ClampedRandom order: maxHeight, distance, then the four raw rand()
        // rerolls of height[1], height[2], height[6] and height[7].
        let max_height = (runtime.rng.clamped_random(21) as f32) * 0.01 + 0.8;
        let distance = (runtime.rng.clamped_random(21) as f32) * 0.1 + 3.0;
        let height_one = rise_angle as f32 - 45.0 + runtime.rng.clamped_random(91) as f32;
        let height_two = rise_angle as f32 - 45.0 + runtime.rng.clamped_random(91) as f32;
        let height_six = rise_angle as f32 - 45.0 + runtime.rng.clamped_random(91) as f32;
        let height_seven = rise_angle as f32 - 45.0 + runtime.rng.clamped_random(91) as f32;

        let index = runtime.launch(PrimitiveId::WanderingFlames, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = duration;
        state.texture = Some(self.name);
        state.size = 0.0;

        for slot in &mut state.slots {
            slot.life = 0;
        }

        let x = position[0];
        let z = position[2];
        let y = position[1] - 5.0;

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.max_height = max_height;
        slot.distance = distance;
        slot.process = 0;
        slot.rise_angle = rise_angle;

        slot.vec_b_pre = [x - position[0], y - position[1], z - position[2]];
        slot.height[0] = rise_angle as f32;
        slot.height[1] = height_one;
        slot.height[2] = height_two;
        slot.height[3] = 180.0;
        slot.height[4] = 180.0;
        slot.height[12] = 0.0;

        slot.vec_b_now = [x - position[0], y - position[1], z - position[2]];
        slot.height[5] = rise_angle as f32;
        slot.height[6] = height_six;
        slot.height[7] = height_seven;
        slot.height[8] = 180.0;
        slot.height[9] = 180.0;

        for index in 0..10 {
            if slot.height[index] < 0.0 {
                slot.height[index] += 360.0;
            }

            if slot.height[index] >= 360.0 {
                slot.height[index] -= 360.0;
            }
        }

        slot.height[13] = 0.0;
        slot.flag1[0] = self.flag;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    fn close(left: f32, right: f32) -> bool {
        (left - right).abs() < 1e-5
    }

    #[test]
    fn steal_spawns_ten_gravity_particles_on_frame_zero() {
        let mut effect = Steal::default();
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 341);

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.sounds, vec!["effect\\EF_Steal.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 10);

        for primitive in &runtime.primitives {
            let state = &primitive.state;

            assert_eq!(state.count, PrimitiveId::Particle3dGravity);
            assert_eq!(state.count.id(), 6);
            assert_eq!(state.duration, 50);
            assert_eq!(state.fade_out_cnt, 50 - 50 / 3);
            assert_eq!(state.sprite, Some("이팩트\\particle7.spr"));
            assert_eq!(state.action, Some("이팩트\\particle7.act"));
            assert!((0.5..1.0).contains(&state.speed));
            assert!((-1.5..=-0.6).contains(&state.grav_speed));
            assert!((1.0..2.0).contains(&state.size));
            assert!(close(state.accel, -(state.speed / 50.0) * 0.5));
            assert!(close(state.grav_accel, -(state.grav_speed / 50.0) * 2.0));
            assert!(close(state.size_speed, -(state.size / 50.0)));
            assert!(close(state.delta_pos2[1], state.delta_pos3[1] - 10.0));
            assert_eq!(state.delta_pos2[0], state.delta_pos3[0]);
            assert_eq!(state.delta_pos2[2], state.delta_pos3[2]);
        }
    }

    #[test]
    fn steal_consumes_the_rng_in_the_official_order_for_every_particle() {
        let mut effect = Steal::default();
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 341);

        assert!(effect.on_process(&mut runtime));

        let angles = Angles::shared();
        let mut expected = crate::world::effect::port::runtime::EffectRng::new(341);

        for (index, primitive) in runtime.primitives.iter().enumerate() {
            let longitude = expected.clamped_random(360) as f32;
            let latitude = (expected.clamped_random(100) - 50) as f32;
            let distance = (expected.clamped_random(4) + 4) as f32;
            let speed = (expected.clamped_random(50) + 50) as f32 / 100.0;
            let grav_speed = -((expected.clamped_random(90) + 60) as f32) / 100.0;
            let size = (expected.clamped_random(100) + 100) as f32 / 100.0;

            let mut transform = Matrix::default();
            transform.make_y_rotation(angles.radian(longitude as i32));
            transform.append_x_rotation(angles.radian(latitude as i32));
            let forward = matrix_mult([0.0, 0.0, distance], &transform.values);

            let mut final_transform = Matrix::default();
            final_transform.make_y_rotation(angles.radian(longitude as i32));

            let state = &primitive.state;
            assert_eq!(state.longitude, longitude, "particle {index}");
            assert_eq!(state.latitude, latitude, "particle {index}");
            assert_eq!(state.delta_pos3, forward, "particle {index}");
            assert_eq!(state.transform, final_transform, "particle {index}");
            assert_eq!(state.speed, speed, "particle {index}");
            assert_eq!(state.grav_speed, grav_speed, "particle {index}");
            assert_eq!(state.size, size, "particle {index}");
        }
    }

    #[test]
    fn steal_does_nothing_after_frame_zero() {
        let mut effect = Steal::default();
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 341);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 10);

        step(&mut effect, &mut runtime);
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 10);
        assert_eq!(runtime.sounds.len(), 1);
    }

    #[test]
    fn steal_matches_the_official_lifetime() {
        assert_eq!(Steal::default().lifetime(), 50);
    }

    #[test]
    fn fork_bolt_spawns_on_frames_zero_ten_and_twenty_only() {
        let mut effect = TexturedForkBolt::new("effect\\melody_a.bmp", 0, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 7);

        for frame in 0..31 {
            assert_eq!(runtime.frame, frame);
            assert!(effect.on_process(&mut runtime));
            runtime.frame += 1;
        }

        assert_eq!(runtime.primitives.len(), 3);
    }

    #[test]
    fn fork_bolt_primitive_matches_the_official_values() {
        let mut effect = TexturedForkBolt::new("effect\\melody_a.bmp", 0, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 7);
        runtime.target_offset = [0.0, 0.0, 10.0];

        assert!(effect.on_process(&mut runtime));

        let mut expected = crate::world::effect::port::runtime::EffectRng::new(7);
        let max_height = (expected.clamped_random(21) as f32) * 0.01 + 0.8;
        let distance = (expected.clamped_random(21) as f32) * 0.1 + 3.0;
        let height_one = 91.0 - 45.0 + expected.clamped_random(91) as f32;
        let height_two = 91.0 - 45.0 + expected.clamped_random(91) as f32;
        let height_six = 91.0 - 45.0 + expected.clamped_random(91) as f32;
        let height_seven = 91.0 - 45.0 + expected.clamped_random(91) as f32;

        assert_eq!(runtime.primitives.len(), 1);
        let state = &runtime.primitives[0].state;

        assert_eq!(state.count, PrimitiveId::WanderingFlames);
        assert_eq!(state.count.id(), 155);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.duration, 300);
        assert_eq!(state.texture, Some("effect\\melody_a.bmp"));
        assert_eq!(state.size, 0.0);

        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        assert_eq!(slot.max_height, max_height);
        assert_eq!(slot.distance, distance);
        assert_eq!(slot.process, 0);
        // Get_Angle(0, 10) = 91, not 90: the client's table uses the literal
        // 3.141592f, so g_cosTable[90] is +3.27e-7 and the >= test only
        // matches index 91 (`ClientAngles.cpp:24-34`, `EffectPrimitiveState.cpp:51-70`).
        assert_eq!(slot.rise_angle, 91);
        assert_eq!(slot.height[0], 91.0);
        assert_eq!(slot.height[1], height_one);
        assert_eq!(slot.height[2], height_two);
        assert_eq!(slot.height[3], 180.0);
        assert_eq!(slot.height[4], 180.0);
        assert_eq!(slot.height[5], 91.0);
        assert_eq!(slot.height[6], height_six);
        assert_eq!(slot.height[7], height_seven);
        assert_eq!(slot.height[8], 180.0);
        assert_eq!(slot.height[9], 180.0);
        assert_eq!(slot.height[12], 0.0);
        assert_eq!(slot.height[13], 0.0);
        assert_eq!(slot.flag1[0], 0);
        // x, y and z are all measured from the same render.position.
        assert_eq!(slot.vec_b_pre, [0.0, -5.0, 0.0]);
        assert_eq!(slot.vec_b_now, [0.0, -5.0, 0.0]);

        for index in 0..10 {
            assert!((0.0..360.0).contains(&slot.height[index]));
        }

        for slot in &state.slots[1..] {
            assert_eq!(slot.life, 0);
        }
    }

    #[test]
    fn fork_bolt_stores_the_dispatch_flag_in_slot_zero() {
        let mut effect = TexturedForkBolt::new("effect\\melody_b.bmp", 1, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 7);

        assert!(effect.on_process(&mut runtime));

        let state = &runtime.primitives[0].state;
        assert_eq!(state.slots[0].flag1[0], 1);
        assert_eq!(state.texture, Some("effect\\melody_b.bmp"));
    }

    #[test]
    fn fork_bolt_keeps_the_zero_vector_get_angle_fallback() {
        let mut effect = TexturedForkBolt::new("effect\\melody_a.bmp", 0, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 7);

        // position and deltaPos are both zero: Get_Angle's table loop never
        // matches because the normalised vector is NaN, and the original
        // leaves the caller's value at 180 (`EffectPrimitiveState.cpp:51-70`).
        assert!(effect.on_process(&mut runtime));

        let slot = &runtime.primitives[0].state.slots[0];
        assert_eq!(slot.rise_angle, 180);
        assert_eq!(slot.height[0], 180.0);
    }

    #[test]
    fn fork_bolt_matches_the_official_lifetime() {
        assert_eq!(TexturedForkBolt::new("effect\\melody_a.bmp", 0, 300).lifetime(), 300);
    }
}
