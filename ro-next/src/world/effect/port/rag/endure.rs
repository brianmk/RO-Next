//! `CRagEffect::Endure` - `RagEffect.cpp:224-284`, effect id `EF_ENDURE`,
//! lifetime 80 frames.
//!
//! Frame 0 spawns the big shrinking `endure.tga` texture; every frame below 50
//! adds a streak (`alpha_down.tga`) flying outward along a random heading.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `EF_ENDURE` lifetime in frames.
pub const LIFETIME: i32 = 80;

#[derive(Debug, Default)]
pub struct Endure {
    spawned: bool,
}

impl PortedEffect for Endure {
    fn lifetime(&self) -> i32 {
        LIFETIME
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let angles = Angles::shared();

        if runtime.frame == 0 {
            self.spawned = true;
            runtime.play_sound("effect\\EF_Endure.wav");

            let duration = runtime.lifetime;
            let body = runtime.launch(PrimitiveId::Texture2d, [0.0, 0.0, 0.0]);
            {
                let state = runtime.primitive_mut(body);
                state.duration = duration;
                state.pattern |= 2;
                state.delta_pos2[1] = -20.0;
                state.width_size = 150.0;
                state.height_size = 150.0;
                state.width_speed = -5.0;
                state.height_speed = -5.0;
                state.ch_cnt = (state.duration - 35) as u32;
                state.width_accel = -state.width_speed / state.ch_cnt as f32;
                state.height_accel = -state.height_speed / state.ch_cnt as f32;
                state.alpha = 0.0;
                state.alpha_speed = state.max_alpha / 20.0;
                // `duration - duration / 3` with integer division: 80 - 26.
                state.fade_out_cnt = state.duration - state.duration / 3;
            }
        }

        if runtime.frame >= 50 {
            return true;
        }

        let heading = runtime.rng.clamped_random(360) as f32;
        let width_size = (runtime.rng.clamped_random(100) + 60) as f32 / 100.0;
        let height_size = (runtime.rng.clamped_random(20) + 30) as f32;
        let radius = (runtime.rng.clamped_random(40) + 100) as f32;

        let streak = runtime.launch(PrimitiveId::Texture2d, [0.0, 0.0, 0.0]);
        {
            let state = runtime.primitive_mut(streak);
            state.duration = 40;
            state.roll = heading;
            state.longitude = heading;
            state.speed = -4.0;
            state.accel = -(state.speed / state.duration as f32);
            state.width_size = width_size;
            state.height_size = height_size;
            state.alpha = 0.0;
            state.alpha_speed = state.max_alpha / 20.0;
            state.fade_out_cnt = state.duration - state.duration / 3;

            state.delta_pos2[0] = angles.sine(state.longitude as i32) * radius;
            state.delta_pos2[1] = angles.cosine(state.longitude as i32) * -radius;
            state.delta_pos2[1] -= 20.0;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    const EPSILON: f32 = 1e-4;

    #[test]
    fn frame_zero_spawns_the_shrinking_body_and_one_streak() {
        let mut effect = Endure::default();
        let mut runtime = EffectRuntime::new(LIFETIME, [320.0, 240.0], 9);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_Endure.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 2);

        let body = &runtime.primitives[0].state;
        assert_eq!(body.count, PrimitiveId::Texture2d);
        assert_eq!(body.duration, 80);
        assert_eq!(body.pattern & 2, 2);
        assert_eq!(body.delta_pos2[1], -20.0);
        // Spawned at 150 and already shrunk by one motion step.
        let accel = 5.0 / 45.0;
        let expected = 150.0 + (-5.0 + accel);
        assert!((body.width_size - expected).abs() < EPSILON);
        assert!((body.height_size - expected).abs() < EPSILON);
        assert_eq!(body.ch_cnt, 45);
        assert!((body.width_accel - accel).abs() < EPSILON);
        assert_eq!(body.fade_out_cnt, 54);

        let streak = &runtime.primitives[1].state;
        assert_eq!(streak.duration, 40);
        // Spawned at -4 and accelerated by one step (+0.1).
        assert!((streak.speed + 3.9).abs() < EPSILON);
        assert!((streak.accel - 0.1).abs() < EPSILON);
        assert!(streak.width_size >= 0.6 && streak.width_size < 1.6);
        assert!(streak.height_size >= 30.0 && streak.height_size < 50.0);
    }

    #[test]
    fn one_streak_per_frame_below_fifty() {
        let mut effect = Endure::default();
        let mut runtime = EffectRuntime::new(LIFETIME, [0.0, 0.0], 9);

        // Frames 0..49 spawn, frame 50 does not.
        for _ in 0..51 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 51);
    }
}
