//! `CRagEffect::EnchantPoison2` - `RagEffect.cpp:13241-13272`.
//!
//! `EF_EDP`: a `particle3.spr` mote every three frames with the enchant-poison
//! wave on frames 0/5/11/18/26/40.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `g_depthAverageHalf`.
const DEPTH_AVERAGE_HALF: f32 = 0.5;

#[derive(Debug, Default)]
pub struct EnchantPoisonCast {}

impl PortedEffect for EnchantPoisonCast {
    fn lifetime(&self) -> i32 {
        120
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if matches!(frame, 0 | 5 | 11 | 18 | 26 | 40) {
            runtime.play_sound("effect\\assasin_enchantpoison.wav");
        }

        if frame % 3 != 0 {
            return true;
        }

        let angles = Angles::shared();

        // RNG order: rotation, forward distance, speed, size.
        let rotation = runtime.rng.clamped_random(360);
        let forward = (runtime.rng.clamped_random(6) + 2) as f32;
        let speed = (runtime.rng.clamped_random(50) + 30) as f32 / 100.0;
        let size = (runtime.rng.clamped_random(20) + 20) as f32 / 100.0;

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
        state.sprite = Some("이팩트\\particle3.spr");
        state.action = Some("이팩트\\particle3.act");
        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn edp_drips_a_mote_every_three_frames() {
        let mut effect = EnchantPoisonCast::default();
        let mut runtime = EffectRuntime::new(120, [0.0, 0.0], 411);

        for _ in 0..6 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2, "frames 0 and 3");
        assert_eq!(
            runtime.sounds,
            vec![
                "effect\\assasin_enchantpoison.wav".to_owned(),
                "effect\\assasin_enchantpoison.wav".to_owned()
            ],
            "frames 0 and 5 play the wave"
        );

        let mote = &runtime.primitives[0].state;
        assert_eq!(mote.count, PrimitiveId::Particle3d);
        assert_eq!(mote.duration, 40);
        assert_eq!(mote.sprite, Some("이팩트\\particle3.spr"));
        assert!((0.3..0.8).contains(&mote.speed));
        assert!((0.2..0.4).contains(&mote.size));
        assert!((mote.roll_speed - 3.0).abs() < 1e-6);
        assert_eq!(mote.fade_out_cnt, 32);
    }
}
