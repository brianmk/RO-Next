//! `CRagEffect::Detoxication` - `RagEffect.cpp:13187-13211`, effect id
//! `EF_DETOXICATION`, lifetime 100 frames.
//!
//! A `particle2.spr` mote every five frames, decelerated by
//! `g_depthAverageHalf`. (The low-effect variant emits every 15 frames; low
//! mode is out of scope.)

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `g_depthAverageHalf` - `MapSkyMgr.cpp:27`.
const DEPTH_AVERAGE_HALF: f32 = 0.5;

#[derive(Debug, Default)]
pub struct Detoxication {
    spawned: bool,
}

impl PortedEffect for Detoxication {
    fn lifetime(&self) -> i32 {
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame % 5 != 0 {
            return true;
        }

        if runtime.frame == 0 {
            self.spawned = true;
            runtime.play_sound("effect\\EF_Detoxication.wav");
        }

        let angles = Angles::shared();

        // RNG order: rotation, forward distance, speed, size.
        let rotation = runtime.rng.clamped_random(360);
        let forward = (runtime.rng.clamped_random(6) + 2) as f32;
        let speed = (runtime.rng.clamped_random(30) + 30) as f32 / 100.0;
        let size = (runtime.rng.clamped_random(50) + 40) as f32 / 100.0;

        let mote = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(mote);

        state.alpha = 0.0;
        state.duration = 40;
        state.transform.make_y_rotation(angles.radian(rotation));

        let offset = matrix_mult([0.0, 0.0, forward], &state.transform.values);
        state.delta_pos2 = offset;

        state.transform.make_x_rotation(angles.radian(90));
        state.speed = speed;
        state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
        state.size = size;
        state.roll_speed = 3.0;
        state.alpha_speed = state.max_alpha / 10.0;
        state.fade_out_cnt = state.duration - state.duration / 5;
        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn one_mote_every_five_frames() {
        let mut effect = Detoxication::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 151);

        while runtime.frame < 11 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.sounds, vec!["effect\\EF_Detoxication.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 3, "frames 0, 5, 10");

        let mote = &runtime.primitives[0].state;
        assert_eq!(mote.count, PrimitiveId::Particle3d);
        assert_eq!(mote.duration, 40);
        assert!((0.3..0.6).contains(&mote.speed));
        assert!((0.4..0.9).contains(&mote.size));
        assert!((mote.roll_speed - 3.0).abs() < 1e-6);
        assert!((mote.alpha_speed - 254.0 / 10.0).abs() < 1e-4);
        assert_eq!(mote.fade_out_cnt, 32);
    }

    #[test]
    fn effect_runs_for_its_full_lifetime() {
        let mut effect = Detoxication::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 151);

        while step(&mut effect, &mut runtime) {
            if runtime.frame > 200 {
                panic!("did not retire");
            }
        }

        assert_eq!(runtime.frame, 100);
    }
}
