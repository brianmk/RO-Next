//! `CRagEffect::NapalmFragments` - `RagEffect.cpp:10018-10058`.
//!
//! `EF_NAPALMBEAT`: an eight-frame explosion animation (`폭발1..8.TGA`) every
//! five frames until frame 15, at a random offset around the caster.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct NapalmBeat {}

/// `effect\\폭발1.TGA` … `폭발8.TGA`.
static EXPLOSION_FRAMES: [&str; 8] = [
    "effect\\폭발1.TGA",
    "effect\\폭발2.TGA",
    "effect\\폭발3.TGA",
    "effect\\폭발4.TGA",
    "effect\\폭발5.TGA",
    "effect\\폭발6.TGA",
    "effect\\폭발7.TGA",
    "effect\\폭발8.TGA",
];

impl PortedEffect for NapalmBeat {
    fn lifetime(&self) -> i32 {
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame >= 15 {
            return true;
        }

        if frame == 0 {
            runtime.play_sound("effect\\EF_NapalmBeat.wav");
        }

        // `stateCnt % 5 == 0` selects frames 0, 5 and 10.
        if frame % 5 != 0 {
            return true;
        }

        let angles = Angles::shared();

        // RNG order: distance, angle, size.
        let distance = (runtime.rng.clamped_random(50) + 10) as f32;
        let angle = runtime.rng.clamped_random(360) as f32;
        let size = (runtime.rng.clamped_random(40) + 20) as f32;

        let burst = runtime.launch(PrimitiveId::Texture2d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(burst);

        state.duration = 30;
        state.delta_pos2[0] = angles.raw_sine(angle as usize) * distance;
        state.delta_pos2[1] = angles.raw_cosine(angle as usize) * -distance - 20.0;
        state.height_size = size;
        state.width_size = size;
        state.alpha = 0.0;
        state.alpha_speed = state.max_alpha / 10.0;
        state.fade_out_cnt = state.duration - state.duration / 3;
        state.repeat_anim = 0;
        state.anim_speed = 2;
        state.frame_count = 8;
        state.texture_frames = EXPLOSION_FRAMES.to_vec();
        state.texture = EXPLOSION_FRAMES.first().copied();
        state.slots[0].life = 0;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn napalm_beat_bursts_three_times() {
        let mut effect = NapalmBeat::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 551);

        for _ in 0..15 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 3, "frames 0, 5, 10");
        assert_eq!(runtime.sounds, vec!["effect\\EF_NapalmBeat.wav".to_owned()]);

        let burst = &runtime.primitives[0].state;
        assert_eq!(burst.count, PrimitiveId::Texture2d);
        assert_eq!(burst.duration, 30);
        assert_eq!(burst.frame_count, 8);
        assert_eq!(burst.texture_frames.len(), 8);
        assert_eq!(burst.texture, Some("effect\\폭발1.TGA"));
        assert_eq!(burst.anim_speed, 2);
        assert_eq!(burst.repeat_anim, 0);
        assert_eq!(burst.fade_out_cnt, 20);
        assert_eq!(burst.slots[0].life, 0);
        assert!(burst.width_size >= 20.0 && burst.width_size <= 59.0);
        // cos(angle) may be negative, so the vertical offset only shifts by -20.
        assert!(burst.delta_pos2[1] >= -80.0 && burst.delta_pos2[1] <= 40.0);
    }
}
