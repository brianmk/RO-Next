//! `CRagEffect::WaterBall2` - `RagEffect.cpp:1190-1220`, dispatch
//! `:23932-23934`.
//!
//! `EF_WATERBALL2` (117): one twelve-segment `PP_3DPARTICLESPLINE` water
//! sprite that arcs from the caster to the target. The target offset is the
//! effect's `deltaPos`; `radius` is the distance to it and the sprite travels
//! `speed`/frame while `gravSpeed` and `latiSpeed` bend the trail.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct WaterBall2 {
    lifetime: i32,
}

impl WaterBall2 {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for WaterBall2 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();
        let target = runtime.target_offset;
        let position = runtime.position;
        let radius = runtime.calc_dist(target[0], target[2]);
        // `CEffectPrim::Face(deltaPos.x, deltaPos.z)`; a fresh primitive's
        // `render.angle` is 0 when the offset is zero.
        let face = if target[0] != position[0] || target[2] != position[2] {
            EffectRuntime::heading(position[0], position[2], target[0], target[2])
        } else {
            0.0
        };

        runtime.play_sound("effect\\wizard_waterball_chulung.wav");

        let spline = runtime.launch(PrimitiveId::ParticleSpline, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spline);

        state.sprite = Some("이팩트\\Waterball.spr");
        state.action = Some("이팩트\\Waterball.act");
        state.duration = 40;
        state.delta_pos2 = [0.0, -10.0, 0.0];
        state.radius = radius;
        state.render_angle = face;
        state.longitude = -state.render_angle;
        state.speed = state.radius / state.duration as f32;
        state.grav_speed = 3.5;
        let rise = -(state.grav_speed / state.duration as f32);
        state.grav_accel = rise + rise;
        state.lati_speed = -2.0;
        let spread = -(state.lati_speed / state.duration as f32);
        state.lati_accel = spread + spread;
        state.roll = 0.0;
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));
        state.delta_pos3 = matrix_mult([0.0, 0.0, 1.0], &state.transform.values);
        state.rot_axis = state.delta_pos3;
        state.size = 0.75;
        state.num_segment = 12;
        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::EffectRuntime;

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 117);
        runtime.position = [10.0, 0.0, 10.0];
        runtime.target_offset = [30.0, 0.0, 10.0];

        runtime
    }

    #[test]
    fn water_ball_two_spawns_one_twelve_segment_spline() {
        let mut body = WaterBall2::new(100);
        let mut runtime = runtime();

        body.on_process(&mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\wizard_waterball_chulung.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::ParticleSpline);
        assert_eq!(state.count.id(), 8);
        assert_eq!(state.duration, 40);
        assert_eq!(state.delta_pos2, [0.0, -10.0, 0.0]);
        assert!((state.radius - 20.0).abs() < 1e-5);
        assert!((state.speed - 0.5).abs() < 1e-5);
        assert_eq!(state.grav_speed, 3.5);
        assert!((state.grav_accel + 0.175).abs() < 1e-5);
        assert_eq!(state.lati_speed, -2.0);
        assert!((state.lati_accel - 0.1).abs() < 1e-5);
        assert_eq!(state.roll, 0.0);
        assert_eq!(state.size, 0.75);
        assert_eq!(state.num_segment, 12);
        assert_eq!(state.sprite, Some("이팩트\\Waterball.spr"));
        assert_eq!(state.action, Some("이팩트\\Waterball.act"));
        // Facing east (+x) from the anchor.
        assert!((state.longitude + 90.0).abs() < 1.0);
    }

    #[test]
    fn water_ball_two_only_spawns_on_frame_zero() {
        let mut body = WaterBall2::new(100);
        let mut runtime = runtime();

        runtime.frame = 1;
        body.on_process(&mut runtime);

        assert!(runtime.primitives.is_empty());
    }
}
