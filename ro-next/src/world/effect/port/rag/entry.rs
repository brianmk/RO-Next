//! `CRagEffect::Entry` - `RagEffect.cpp:2191-2230`.
//!
//! `EF_ENTRY`: two blue cylinders on frame 0 - a rising column and a shrinking
//! spiral - both with the official speed/accel constants.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct Entry {
    lifetime: i32,
}

impl Entry {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Entry {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();

        // Column: latitude/longitude are both zero at this point.
        let column = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(column);

        state.duration = 55;
        state.transform.make_x_rotation(angles.radian(0));
        state.transform.append_y_rotation(angles.radian(0));
        state.accel = -0.00007;
        state.long_speed = -10.0;
        state.outer_size = 6.0;
        state.outer_speed = 0.08;
        state.inner_size = 5.0;
        state.height_size = 6.5;
        state.height_speed = -0.1;
        state.alpha = 5.0;
        state.max_alpha = 245.0;
        state.alpha_speed = 10.0;
        state.fade_out_cnt = state.duration - state.duration / 5;
        state.texture = Some("effect\\ring_blue.tga");

        // Spiral: the shrinking ring.
        let spiral = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spiral);

        state.duration = 55;
        state.arc_angle = 45.0;
        state.long_speed = 10.0;
        state.outer_size = 4.5;
        state.inner_size = 4.5;
        state.height_speed = 2.5;
        let accel = -(state.height_speed / state.duration as f32);
        state.height_accel = accel + accel;
        state.alpha = 160.0;
        state.fade_out_cnt = state.duration - state.duration / 3;
        state.texture = Some("effect\\ring_blue.tga");

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn entry_spawns_the_two_blue_cylinders() {
        let mut effect = Entry::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 591);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 2);

        let column = &runtime.primitives[0].state;
        assert_eq!(column.count, PrimitiveId::Cylinder3d);
        assert_eq!(column.duration, 55);
        assert_eq!(column.long_speed, -10.0);
        // One cylinder step applied outerSpeed.
        assert!((column.outer_size - 6.08).abs() < 1e-4);
        assert_eq!(column.outer_speed, 0.08);
        assert_eq!(column.inner_size, 5.0);
        // One step applied heightSpeed (-0.1).
        assert!((column.height_size - 6.4).abs() < 1e-4);
        // Spawned at 5 with alphaSpeed 10.
        assert!((column.alpha - 15.0).abs() < 1e-4);
        assert_eq!(column.max_alpha, 245.0);
        assert_eq!(column.fade_out_cnt, 44);

        let spiral = &runtime.primitives[1].state;
        assert_eq!(spiral.arc_angle, 45.0);
        assert_eq!(spiral.long_speed, 10.0);
        assert_eq!(spiral.outer_size, 4.5);
        assert_eq!(spiral.alpha, 160.0);
        assert_eq!(spiral.fade_out_cnt, 37);
        assert!(spiral.height_accel < 0.0);
    }
}
