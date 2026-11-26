//! `CRagEffect::GrimToothAtk` - `RagEffect.cpp:2050-2091`.
//!
//! `EF_GRIMTOOTHATK`: three stone spikes in a triangle on frame 0, each with a
//! fixed `deltaPos2`, `speed 3.5`, `accel 0.001`, `chCnt 10`.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct GrimToothAtk {}

impl PortedEffect for GrimToothAtk {
    fn lifetime(&self) -> i32 {
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();
        let position = runtime.position;

        for index in 0..3usize {
            let delta = match index {
                0 => [0.0, 40.0, -12.0],
                1 => [12.0, 40.0, 6.0],
                _ => [-12.0, 40.0, 6.0],
            };

            let spike = runtime.launch(PrimitiveId::QuadHorn3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spike);

            state.pattern |= 1;
            state.render_flag |= 0x200;
            state.duration = 1000;
            state.delta_pos2 = delta;
            state.size = 0.9;
            state.height_size = 25.0;
            state.longitude = ((6 - index) * 120) as f32;
            state.latitude = 75.0;
            state.transform.make_x_rotation(angles.radian(75));
            state.transform.append_y_rotation(angles.radian(state.longitude as i32));
            state.rot_transform = state.transform;
            state.speed = 3.5;
            state.accel = 0.001;
            state.ch_cnt = 10;
            state.fade_out_cnt = state.duration - 10;
            state.texture = Some("effect\\stone.bmp");
            state.position = position;

            let delta = state.delta_pos2;
            runtime.delta_pos3[0] -= delta[0];
            runtime.delta_pos3[1] -= delta[1];
            runtime.delta_pos3[2] -= delta[2];
        }

        // The original sets the effect duration to 1000 here.
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn grim_tooth_atk_spawns_three_spikes() {
        let mut effect = GrimToothAtk::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 531);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);

        let first = &runtime.primitives[0].state;
        assert_eq!(first.count, PrimitiveId::QuadHorn3d);
        assert_eq!(first.pattern & 1, 1);
        assert_eq!(first.render_flag & 0x200, 0x200);
        assert_eq!(first.duration, 1000);
        assert_eq!(first.delta_pos2, [0.0, 40.0, -12.0]);
        assert_eq!(first.size, 0.9);
        assert_eq!(first.height_size, 25.0);
        assert_eq!(first.latitude, 75.0);
        assert_eq!(first.longitude, 720.0);
        assert_eq!(first.ch_cnt, 10);
        assert_eq!(first.fade_out_cnt, 990);
        assert_eq!(first.texture, Some("effect\\stone.bmp"));

        assert_eq!(runtime.primitives[1].state.delta_pos2, [12.0, 40.0, 6.0]);
        assert_eq!(runtime.primitives[1].state.longitude, 600.0);
        assert_eq!(runtime.primitives[2].state.longitude, 480.0);
    }
}
