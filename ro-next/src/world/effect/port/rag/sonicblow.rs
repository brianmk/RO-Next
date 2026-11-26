//! `EF_SONICBLOW` - inline body in `CRagEffect::OnProcess2`,
//! `RagEffect.cpp:23801-23821`. Lifetime 40 frames (`EffectTypes.cpp:130`).
//!
//! One `PP_2DCIRCLE` expanding at 7 px/frame with a heavy deceleration
//! (`-(radiusSpeed / (duration + 40)) * 2`), anchored 20 px above the target.
//! `fadeOutCnt = 0` makes the ring fade from its very first frame.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `EF_SONICBLOW` lifetime in frames.
pub const LIFETIME: i32 = 40;

#[derive(Debug, Default)]
pub struct SonicBlow {
    spawned: bool,
}

impl PortedEffect for SonicBlow {
    fn lifetime(&self) -> i32 {
        LIFETIME
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        self.spawned = true;
        runtime.play_sound("effect\\EF_StoneCurse.wav");

        let duration = runtime.lifetime;
        let ring = runtime.launch(PrimitiveId::Circle2d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.duration = duration;
        state.fade_out_cnt = 0;
        state.delta_pos2[1] = -20.0;
        state.arc_angle = 10.0;
        state.inner_size = 10.0;
        state.radius_speed = 7.0;
        // `ring.radiusAccel = -(ring.radiusSpeed / (duration + 40)) * 2`
        state.radius_accel = -(state.radius_speed / (state.duration + 40) as f32) * 2.0;
        state.texture = Some("effect\\alpha_center.tga");
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, run_to_completion, step};

    const EPSILON: f32 = 1e-4;

    #[test]
    fn spawns_one_fast_ring() {
        let mut effect = SonicBlow::default();
        let mut runtime = EffectRuntime::new(LIFETIME, [0.0, 0.0], 3);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_StoneCurse.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Circle2d);
        assert_eq!(state.duration, 40);
        assert_eq!(state.fade_out_cnt, 0);
        assert_eq!(state.delta_pos2[1], -20.0);
        assert_eq!(state.arc_angle, 10.0);
        assert_eq!(state.inner_size, 10.0);
        assert!((state.radius_accel - -(7.0 / 80.0) * 2.0).abs() < EPSILON);
    }

    #[test]
    fn ring_fades_from_the_first_frame() {
        let mut effect = SonicBlow::default();
        let mut runtime = EffectRuntime::new(LIFETIME, [0.0, 0.0], 3);

        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert!(state.alpha_speed < 0.0, "fade-out starts at stateCnt 0");
        assert!(state.alpha < 254.0);
    }

    #[test]
    fn effect_retires_after_forty_frames() {
        let mut effect = SonicBlow::default();
        let mut runtime = EffectRuntime::new(LIFETIME, [0.0, 0.0], 3);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 40);
    }
}
