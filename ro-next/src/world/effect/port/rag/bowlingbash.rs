//! `CRagEffect::BowlingBash` - `RagEffect.cpp:2277-2321`, effect id
//! `EF_BOWLINGBASH`, lifetime 250 frames.
//!
//! A blue cylinder arc (`PP_3DCYLINDER`) launched at frames 0 and 5, plus a
//! yellow ground ring (`PP_3DCIRCLE`) on frame 0 whose acceleration switches at
//! `changePoint 20`.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct BowlingBash {
    spawned: bool,
}

impl PortedEffect for BowlingBash {
    fn lifetime(&self) -> i32 {
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 && runtime.frame != 5 {
            return true;
        }

        self.spawned = true;

        let angles = Angles::shared();
        let frame = runtime.frame;
        let mut facing = (180.0f32 - runtime.parent_angle) as i32;

        let cylinder = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        {
            let state = runtime.primitive_mut(cylinder);
            state.duration = 20 - frame;
            state.render_flag |= 0x200;

            if facing < 0 {
                facing += 360;
            }

            state.longitude = (facing + frame * 20) as f32;
            state.transform.make_x_rotation(angles.radian(state.latitude as i32));
            state.transform.append_y_rotation(angles.radian(state.longitude as i32));
            state.outer_size = 8.0;
            state.outer_speed = 0.5;
            state.outer_accel = -0.03;
            state.inner_size = 3.0;
            state.inner_speed = 0.5;
            state.inner_accel = -0.03;
            state.height_size = 3.5;
            state.alpha = (frame as f32 * -7.0) + 240.0;

            let duration = state.duration;
            state.fade_out_cnt = duration - duration / 2;
            state.texture = Some("effect\\ring_blue.tga");
        }

        if frame == 0 {
            let ring = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(ring);
            state.duration = 50;
            state.render_flag |= 0x200;
            state.latitude = 90.0;
            state.radius = 8.0;
            state.radius_speed = 0.7;
            state.radius_accel = -(state.radius_speed / 30.0);
            state.inner_size = 5.0;
            state.change_point = 20;
            state.change_accel = 0.0;
            state.change_speed = 0.08;
            state.alpha = 45.0;
            state.fade_out_cnt = 35;
            state.init_segment();
            state.texture = Some("effect\\ring_yellow.tga");
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    const EPSILON: f32 = 1e-4;

    #[test]
    fn frame_zero_spawns_cylinder_and_ring() {
        let mut effect = BowlingBash::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 41);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 2);

        let cylinder = &runtime.primitives[0].state;
        assert_eq!(cylinder.render_flag & 0x200, 0x200);
        assert_eq!(cylinder.count, PrimitiveId::Cylinder3d);
        assert_eq!(cylinder.duration, 20);
        assert_eq!(cylinder.longitude, 180.0, "facing 180 with parent angle 0");
        // Spawned at 8 and already integrated one frame (+0.47).
        assert!((cylinder.outer_size - 8.47).abs() < 1e-3);
        assert!((cylinder.outer_speed - 0.47).abs() < 1e-3);
        assert_eq!(cylinder.outer_accel, -0.03);
        assert!((cylinder.inner_size - 3.47).abs() < 1e-3);
        assert_eq!(cylinder.height_size, 3.5);
        assert_eq!(cylinder.alpha, 240.0);
        assert_eq!(cylinder.fade_out_cnt, 10);

        let ring = &runtime.primitives[1].state;
        assert_eq!(ring.count, PrimitiveId::Circle3d);
        assert_eq!(ring.duration, 50);
        // Spawned at 8 and integrated one frame: radiusSpeed 0.7 - 0.7/30.
        let speed = 0.7 - 0.7f32 / 30.0;
        assert!((ring.radius - (8.0 + speed)).abs() < EPSILON);
        assert!((ring.radius_accel - -(0.7f32 / 30.0)).abs() < EPSILON);
        assert_eq!(ring.change_point, 20);
        assert_eq!(ring.change_speed, 0.08);
        assert_eq!(ring.alpha, 45.0);
        assert_eq!(ring.fade_out_cnt, 35);
    }

    #[test]
    fn frame_five_adds_only_the_cylinder() {
        let mut effect = BowlingBash::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 41);

        while runtime.frame < 6 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 3);

        let second = &runtime.primitives[2].state;
        assert_eq!(second.count, PrimitiveId::Cylinder3d);
        assert_eq!(second.duration, 15);
        assert_eq!(second.alpha, 205.0);
        assert_eq!(second.fade_out_cnt, 8);
        // 180 + 5 * 20 = 280.
        assert_eq!(second.longitude, 280.0);
    }
}
