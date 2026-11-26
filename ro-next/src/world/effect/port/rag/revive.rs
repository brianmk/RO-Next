//! `EF_REVIVE` - inline body in `CRagEffect::OnProcess`,
//! `RagEffect.cpp:24061-24090`. Lifetime 250 frames.
//!
//! A yellow cylinder every 25 frames; the frame-0 cylinder is the long-lived
//! base (effect duration, `fadeOutCnt = duration - duration / 10`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct Revive {
    spawned: bool,
}

impl PortedEffect for Revive {
    fn lifetime(&self) -> i32 {
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame % 25 != 0 {
            return true;
        }

        self.spawned = true;

        let frame = runtime.frame;
        let effect_duration = runtime.lifetime;

        let ring = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.duration = 60;
        state.long_speed = 1.5;
        state.long_accel = 0.03;
        state.outer_size = 10.5;
        state.inner_speed = -0.12;
        state.inner_accel = 0.003;
        state.outer_speed = -0.12;
        state.outer_accel = 0.003;
        state.inner_size = 7.5;
        state.height_size = 7.5;
        state.alpha = 240.0;
        state.fade_out_cnt = state.duration - state.duration / 2;

        if frame == 0 {
            state.duration = effect_duration;
            state.inner_speed = 0.0;
            state.inner_accel = 0.0;
            state.long_speed = 1.5;
            state.outer_size = 11.0;
            state.inner_size = 7.5;
            state.outer_speed = 0.0;
            state.outer_accel = 0.0;
            state.fade_out_cnt = state.duration - state.duration / 10;
        }

        state.texture = Some("effect\\ring_yellow.tga");

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn frame_zero_uses_the_long_base_cylinder() {
        let mut effect = Revive::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 111);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let base = &runtime.primitives[0].state;
        assert_eq!(base.count, PrimitiveId::Cylinder3d);
        assert_eq!(base.duration, 250);
        assert_eq!(base.outer_size, 11.0);
        assert_eq!(base.inner_size, 7.5);
        assert_eq!(base.outer_speed, 0.0);
        assert_eq!(base.fade_out_cnt, 225);
        assert_eq!(base.texture, Some("effect\\ring_yellow.tga"));
    }

    #[test]
    fn later_cylinders_every_twenty_five_frames_are_short_lived() {
        let mut effect = Revive::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 111);

        // Stop right after the frame-25 spawn so the values are one step old.
        while runtime.frame < 26 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2, "frames 0 and 25");

        let second = &runtime.primitives[1].state;
        assert_eq!(second.duration, 60);
        assert!((second.outer_size - (10.5 - 0.12 + 0.003)).abs() < 1e-3);
        assert!((second.inner_speed - (-0.12 + 0.003)).abs() < 1e-6);
        assert_eq!(second.fade_out_cnt, 30);
    }
}
