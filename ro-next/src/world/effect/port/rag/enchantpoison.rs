//! `CRagEffect::PoisonAttack(flag)` used by `EF_ENCHANTPOISON` -
//! `RagEffect.cpp:13276-13338` and the dispatch at `:23831`.
//!
//! Two sounds on frame 0, then a `particle3.spr` mote every five frames.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `g_depthAverageHalf` - `MapSkyMgr.cpp:27`.
const DEPTH_AVERAGE_HALF: f32 = 0.5;

#[derive(Debug, Default)]
pub struct EnchantPoison {
    spawned: bool,
}

impl PortedEffect for EnchantPoison {
    fn lifetime(&self) -> i32 {
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            self.spawned = true;
            runtime.play_sound("effect\\assasin_enchantpoison.wav");
            runtime.play_sound("effect\\EF_PoisonAttack.wav");
        }

        if runtime.frame % 5 != 0 {
            return true;
        }

        let angles = Angles::shared();

        // RNG order: rotation, forward distance, speed, size.
        let rotation = runtime.rng.clamped_random(360);
        let forward = (runtime.rng.clamped_random(6) + 2) as f32;
        let speed = (runtime.rng.clamped_random(30) + 30) as f32 / 100.0;
        let size = (runtime.rng.clamped_random(50) + 40) as f32 / 100.0;

        let mote = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(mote);

        state.duration = 40;
        state.transform.make_y_rotation(angles.radian(rotation));

        let offset = matrix_mult([0.0, 0.0, forward], &state.transform.values);
        state.delta_pos2 = offset;

        state.transform.make_x_rotation(angles.radian(90));
        state.speed = speed;
        state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
        state.size = size;
        state.roll_speed = 3.0;
        state.alpha = 0.0;
        state.alpha_speed = state.max_alpha / 10.0;
        state.fade_out_cnt = state.duration - state.duration / 5;
        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn two_sounds_then_a_mote_every_five_frames() {
        let mut effect = EnchantPoison::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 131);

        while runtime.frame < 11 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.sounds, vec![
            "effect\\assasin_enchantpoison.wav".to_owned(),
            "effect\\EF_PoisonAttack.wav".to_owned()
        ]);

        // Frames 0, 5, 10.
        assert_eq!(runtime.primitives.len(), 3);

        let mote = &runtime.primitives[0].state;
        assert_eq!(mote.count, PrimitiveId::Particle3d);
        assert_eq!(mote.duration, 40);
        assert!((0.3..0.6).contains(&mote.speed));
        assert!((0.4..0.9).contains(&mote.size));
        assert!((mote.roll_speed - 3.0).abs() < 1e-6);
        assert_eq!(mote.fade_out_cnt, 32);
    }

    #[test]
    fn no_mote_between_the_five_frame_steps() {
        let mut effect = EnchantPoison::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 131);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);
    }
}
