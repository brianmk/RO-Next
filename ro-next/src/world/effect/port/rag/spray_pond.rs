//! `CRagEffect::SprayPond` - `RagEffect.cpp:1135-1186`.
//!
//! `EF_SPRAYPOND`: an eight-segment pond rim on frame 0, `alpha_down` splashes
//! every ten frames and an expanding `alpha_center` ring every twenty.

use super::super::angles::Angles;
use super::super::math::{Matrix, matrix_mult};
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct SprayPond {
    lifetime: i32,
}

impl SprayPond {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for SprayPond {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let angles = Angles::shared();

        if frame == 0 {
            for angle in (0..360).step_by(45) {
                let mut rotation = Matrix::identity();
                rotation.make_y_rotation(angles.radian(((angle as f32) + 22.5) as i32));

                let edge = matrix_mult([0.0, 0.0, 12.2], &rotation.values);

                let rim = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(rim);

                state.pattern = 2;
                state.duration = 130;
                state.transform = rotation;
                state.delta_pos2 = [edge[0], edge[1] - 2.0, edge[2]];

                let mut rot = Matrix::identity();
                rot.make_y_rotation(angles.radian((angle + 90) as f32 as i32 + 22));
                state.rot_transform = rot;

                state.alpha = 120.0;
                state.width_size = 5.0;
                state.height_size = 2.7;
                state.texture = Some("effect\\pond_water.bmp");
            }
        }

        if frame % 10 == 0 {
            for angle in (0..360).step_by(45) {
                let mut rotation = Matrix::identity();
                rotation.make_y_rotation(angles.radian(((angle as f32) + 22.5) as i32));

                let splash = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(splash);

                state.duration = 30;
                state.transform = rotation;
                state.delta_pos2 = matrix_mult([0.0, 0.0, 13.8], &rotation.values);

                let mut rot = Matrix::identity();
                rot.make_z_rotation(angles.radian(90));
                rot.append_y_rotation(angles.radian((angle + 90) as f32 as i32 + 22));
                state.rot_transform = rot;

                state.alpha = 100.0;
                state.fade_out_cnt = 0;
                state.width_size = 5.5;
                state.height_size = 1.0;
                state.texture = Some("effect\\alpha_down.tga");
            }
        }

        if frame % 20 == 0 {
            let ring = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(ring);

            state.duration = 25;
            state.radius = 13.0;
            state.arc_angle = 10.0;
            state.size = 1.25;
            state.latitude = 90.0;
            state.transform.make_x_rotation(angles.radian(90));
            state.radius_speed = 0.25;
            state.radius_accel = -(state.radius_speed / state.duration as f32);
            state.alpha = 100.0;
            state.fade_out_cnt = 0;
            state.init_segment();
            state.texture = Some("effect\\alpha_center.tga");
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn spray_pond_builds_the_rim_and_splashes() {
        let mut effect = SprayPond::new(130);
        let mut runtime = EffectRuntime::new(130, [0.0, 0.0], 581);

        step(&mut effect, &mut runtime);
        // 8 rim segments + 8 splashes + 1 ring on frame 0.
        assert_eq!(runtime.primitives.len(), 17);

        let rim = &runtime.primitives[0].state;
        assert_eq!(rim.count, PrimitiveId::Texture3d);
        assert_eq!(rim.pattern, 2);
        assert_eq!(rim.duration, 130);
        assert_eq!(rim.alpha, 120.0);
        assert_eq!(rim.width_size, 5.0);
        assert_eq!(rim.height_size, 2.7);
        assert_eq!(rim.texture, Some("effect\\pond_water.bmp"));

        let splash = &runtime.primitives[8].state;
        assert_eq!(splash.duration, 30);
        assert_eq!(splash.width_size, 5.5);
        assert_eq!(splash.height_size, 1.0);
        assert_eq!(splash.fade_out_cnt, 0);
        assert_eq!(splash.texture, Some("effect\\alpha_down.tga"));

        let ring = &runtime.primitives[16].state;
        assert_eq!(ring.count, PrimitiveId::Circle3d);
        assert_eq!(ring.duration, 25);
        // The first motion step already expanded the radius.
        assert!(ring.radius > 13.0 && ring.radius < 14.0);
        assert_eq!(ring.arc_angle, 10.0);
        assert_eq!(ring.texture, Some("effect\\alpha_center.tga"));
    }

    #[test]
    fn splashes_repeat_every_ten_frames() {
        let mut effect = SprayPond::new(130);
        let mut runtime = EffectRuntime::new(130, [0.0, 0.0], 581);

        step(&mut effect, &mut runtime);
        let base = runtime.primitives.len();

        while runtime.frame < 10 {
            step(&mut effect, &mut runtime);
        }
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), base + 8, "eight splashes at frame 10");
    }
}
