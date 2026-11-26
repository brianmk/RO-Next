//! `CRagEffect::Bash` - `RagEffect.cpp:1601-1670`, effect id `EF_BASH`, lifetime
//! 40 frames (`EffectTypes.cpp:25`).
//!
//! The reference port: one `PP_2DCIRCLE` (a 100 px screen-space ring that fades
//! in over 6 frames and out from frame 30) plus 20 randomised `PP_2DFLASH`
//! sparks. Everything below - including the RNG call order and the integer
//! constant expressions - is copied from the decompiled body.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `EF_BASH` lifetime in frames.
pub const LIFETIME: i32 = 40;

#[derive(Debug, Default)]
pub struct Bash {
    spawned: bool,
}

impl PortedEffect for Bash {
    fn lifetime(&self) -> i32 {
        LIFETIME
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // The official body recomputes `tlvertX/Y` from the camera every frame;
        // the runtime already carries the projected anchor.
        if runtime.frame != 0 {
            return true;
        }

        self.spawned = true;

        runtime.play_sound("effect\\EF_Bash.wav");

        // `LaunchEffectPrim(4, ...)` - the ring.
        let ring = runtime.launch(PrimitiveId::Circle2d, [0.0, 0.0, 0.0]);
        {
            let state = runtime.primitive_mut(ring);
            state.duration = 40;
            state.pattern |= 1;
            state.delta_pos2[1] = -40.0;
            state.radius = 100.0;
            state.alpha = 0.0;
            state.max_alpha = 170.0;
            state.alpha_speed = state.max_alpha / 6.0;
            state.fade_out_cnt = state.duration - 10;
        }

        // 20 sparks. (The low-effect variant halves this to 10; out of scope.)
        for _ in 0..20 {
            // RNG order must match the original: longitude, longSpeed,
            // arcAngle, heightSize, heightSpeed.
            let longitude = runtime.rng.clamped_random(360) as f32;
            let long_speed = (runtime.rng.clamped_random(60) + 10) as f32 / 10.0;
            let arc_angle = (runtime.rng.clamped_random(25) + 5) as f32 / 10.0;
            let height_size = (runtime.rng.clamped_random(40) + 20) as f32;
            let height_speed = (runtime.rng.clamped_random(30) + 20) as f32 / 10.0;

            let spark = runtime.launch(PrimitiveId::Flash2d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spark);
            state.duration = 40;
            state.delta_pos2[1] = -40.0;
            state.longitude = longitude;
            state.long_speed = long_speed;
            // `current.longAccel = -(current.longSpeed / duration) / 1.5`
            state.long_accel = -(state.long_speed / state.duration as f32) / 1.5;
            state.arc_angle = arc_angle;
            state.height_size = height_size;
            state.height_speed = height_speed;
            state.alpha = 0.0;
            state.max_alpha = 200.0;
            state.alpha_speed = state.max_alpha / 10.0;
            // `duration - duration / 3` with integer division: 40 - 13 = 27.
            state.fade_out_cnt = state.duration - state.duration / 3;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{run_to_completion, step, EffectRuntime};

    fn runtime() -> EffectRuntime {
        EffectRuntime::new(LIFETIME, [320.0, 240.0], 0x1234_5678)
    }

    #[test]
    fn spawns_one_ring_and_twenty_sparks_on_frame_zero() {
        let mut effect = Bash::default();
        let mut runtime = runtime();

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 21);
        assert_eq!(runtime.sounds, vec!["effect\\EF_Bash.wav".to_owned()]);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::Circle2d);
        assert_eq!(ring.duration, 40);
        assert_eq!(ring.pattern & 1, 1);
        assert_eq!(ring.delta_pos2[1], -40.0);
        assert_eq!(ring.radius, 100.0);
        assert_eq!(ring.max_alpha, 170.0);
        assert_eq!(ring.fade_out_cnt, 30);

        for spark in &runtime.primitives[1..] {
            let state = &spark.state;
            assert_eq!(state.count, PrimitiveId::Flash2d);
            assert_eq!(state.duration, 40);
            assert_eq!(state.delta_pos2[1], -40.0);
            assert_eq!(state.max_alpha, 200.0);
            assert_eq!(state.alpha_speed, 20.0);
            assert_eq!(state.fade_out_cnt, 27);
            // Ranges from the original random expressions.
            // The spark already ran one motion step, which adds longSpeed (<= 6.9).
            assert!((-7.0..367.0).contains(&state.longitude), "longitude = {}", state.longitude);
            // 1.0..6.9 at spawn, minus one frame of longAccel (up to -0.115).
            assert!((0.8..7.0).contains(&state.long_speed));
            assert!((0.5..3.0).contains(&state.arc_angle));
            // Spawned in 20..60, then one motion step adds heightSpeed (<= 4.9).
            assert!((20.0..65.0).contains(&state.height_size));
            assert!((2.0..5.0).contains(&state.height_speed));
        }
    }

    #[test]
    fn ring_fades_in_over_six_frames_then_holds() {
        let mut effect = Bash::default();
        let mut runtime = runtime();

        // Frame 0: alpha 0, alphaSpeed 170/6 -> first step yields 28.333...
        step(&mut effect, &mut runtime);
        let ring = &runtime.primitives[0].state;
        assert!((ring.alpha - 170.0 / 6.0).abs() < 1e-4);

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.alpha, 170.0, "clamped at maxAlpha once the ramp is done");
    }

    #[test]
    fn ring_starts_fading_out_at_frame_thirty() {
        let mut effect = Bash::default();
        let mut runtime = runtime();

        while runtime.frame < 30 {
            step(&mut effect, &mut runtime);
        }

        // The primitive's own frame is 30 when the effect frame is 30 (both
        // start at 0 and advance together).
        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.state_cnt, 30);
        assert_eq!(ring.alpha, 170.0);

        // Next step hits `stateCnt == fadeOutCnt` and flips the alpha speed.
        let before = ring.alpha;
        step(&mut effect, &mut runtime);
        let ring = &runtime.primitives[0].state;
        assert!(ring.alpha < before, "alpha decreases after fade-out starts");
    }

    #[test]
    fn effect_retires_after_forty_frames() {
        let mut effect = Bash::default();
        let mut runtime = runtime();

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 40);
        assert_eq!(snapshots.last().unwrap().frame, 40);
        assert_eq!(snapshots[0].primitives.len(), 21);
    }

    #[test]
    fn identical_seed_produces_identical_effects() {
        let run = || {
            let mut effect = Bash::default();
            let mut runtime = runtime();
            run_to_completion(&mut effect, &mut runtime)
        };

        let left = run();
        let right = run();

        assert_eq!(left.len(), right.len());
        for (a, b) in left.iter().zip(right.iter()) {
            assert_eq!(a.primitives.len(), b.primitives.len());
            for (pa, pb) in a.primitives.iter().zip(b.primitives.iter()) {
                assert_eq!(pa.longitude, pb.longitude);
                assert_eq!(pa.height_size, pb.height_size);
                assert_eq!(pa.alpha, pb.alpha);
            }
        }
    }
}
