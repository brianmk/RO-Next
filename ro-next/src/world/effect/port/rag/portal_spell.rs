//! `CRagEffect::Portal` - `RagEffect.cpp:11024-11071`.
//!
//! `EF_PORTAL`: two stacked blue cylinders on frame 0 plus an expanding
//! `PP_CIRCLE3D` ring every 14 frames.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct PortalSpell {
    lifetime: i32,
}

impl PortalSpell {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for PortalSpell {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let lifetime = runtime.lifetime;
        let angles = Angles::shared();

        if frame == 0 {
            for (outer, long_speed, height) in [(3.5f32, 4.0f32, 40.0f32), (4.0, 5.0, 50.0)] {
                let pillar = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(pillar);

                state.duration = lifetime;
                state.arc_angle = 36.0;
                state.outer_size = outer;
                state.inner_size = outer;
                state.long_speed = long_speed;
                state.height_size = height;
                state.alpha = 0.0;
                state.max_alpha = 128.0;
                state.alpha_speed = state.max_alpha / 6.0;
                state.fade_out_cnt = state.duration - 6;
                state.texture = Some("effect\\ring_blue.tga");
            }
        }

        if frame % 14 == 0 {
            let ring = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(ring);

            state.duration = 30;
            state.delta_pos2[1] = -1.0;
            state.inner_size = 8.0;
            state.latitude = 90.0;
            state.transform.make_x_rotation(angles.radian(90));
            state.radius_speed = 1.0;
            state.radius_accel = -(state.radius_speed / state.duration as f32) * 0.5;
            state.fade_out_cnt = state.duration - state.duration / 2;
            state.init_segment();
            state.texture = Some("effect\\ring_blue.tga");
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn portal_stacks_two_cylinders_and_a_ring() {
        let mut effect = PortalSpell::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 481);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);

        let low = &runtime.primitives[0].state;
        assert_eq!(low.count, PrimitiveId::Cylinder3d);
        assert_eq!(low.duration, 100);
        assert_eq!(low.arc_angle, 36.0);
        assert_eq!(low.outer_size, 3.5);
        assert_eq!(low.long_speed, 4.0);
        assert_eq!(low.height_size, 40.0);
        assert_eq!(low.max_alpha, 128.0);
        assert_eq!(low.fade_out_cnt, 94);
        assert_eq!(low.texture, Some("effect\\ring_blue.tga"));

        let high = &runtime.primitives[1].state;
        assert_eq!(high.outer_size, 4.0);
        assert_eq!(high.long_speed, 5.0);
        assert_eq!(high.height_size, 50.0);

        let ring = &runtime.primitives[2].state;
        assert_eq!(ring.count, PrimitiveId::Circle3d);
        assert_eq!(ring.duration, 30);
        assert_eq!(ring.inner_size, 8.0);
        assert_eq!(ring.latitude, 90.0);
        assert!((ring.radius_accel + 1.0 / 30.0 * 0.5).abs() < 1e-6);
        assert_eq!(ring.fade_out_cnt, 15);
    }

    #[test]
    fn rings_repeat_every_fourteen_frames() {
        let mut effect = PortalSpell::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 481);

        step(&mut effect, &mut runtime);
        let after_spawn = runtime.primitives.len();

        for _ in 0..14 {
            step(&mut effect, &mut runtime);
        }

        assert!(runtime.primitives.len() > after_spawn);
    }
}
