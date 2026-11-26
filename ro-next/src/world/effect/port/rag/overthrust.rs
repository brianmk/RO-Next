//! `CRagEffect::StoneCurseRing` used by `EF_OVERTHRUST` -
//! `RagEffect.cpp:10441-10470` and the `case EF_OVERTHRUST` dispatch at `:23834`.
//!
//! A single fast, immediately fading ring plus two sounds. The body overrides
//! the effect duration to 9999 frames, exactly as the original does, so the
//! table lifetime is irrelevant.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct Overthrust {
    spawned: bool,
}

impl PortedEffect for Overthrust {
    fn lifetime(&self) -> i32 {
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        self.spawned = true;
        runtime.play_sound("effect\\black_overthrust.wav");
        runtime.play_sound("effect\\EF_StoneCurse.wav");

        // `this->state.duration = 9999` - the effect keeps itself alive.
        runtime.lifetime = 9999;

        let ring = runtime.launch(PrimitiveId::Circle2d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);
        state.duration = 40;
        state.delta_pos2[1] = -20.0;
        state.arc_angle = 10.0;
        state.inner_size = 10.0;
        state.radius_speed = 7.0;
        let slowdown = -(state.radius_speed / (state.duration + 40) as f32);
        state.radius_accel = slowdown + slowdown;
        state.fade_out_cnt = 0;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    const EPSILON: f32 = 1e-4;

    #[test]
    fn spawns_ring_and_extends_the_effect_lifetime() {
        let mut effect = Overthrust::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 5);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds.len(), 2);
        assert_eq!(runtime.lifetime, 9999);
        assert_eq!(runtime.primitives.len(), 1);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.duration, 40);
        assert_eq!(ring.delta_pos2[1], -20.0);
        assert_eq!(ring.arc_angle, 10.0);
        assert_eq!(ring.inner_size, 10.0);
        assert_eq!(ring.fade_out_cnt, 0);
        assert!((ring.radius_accel - (-(7.0f32 / 80.0) * 2.0)).abs() < EPSILON);
        assert!(ring.alpha_speed < 0.0, "fade starts on frame 0");
    }

    #[test]
    fn survives_past_the_table_lifetime() {
        let mut effect = Overthrust::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 5);

        for _ in 0..300 {
            assert!(step(&mut effect, &mut runtime));
        }
    }
}
