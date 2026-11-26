//! `CRagEffect::GrimTooth` - `RagEffect.cpp:531-583`.
//!
//! `EF_GRIMTOOTH`: a stone burst every three frames along the path to the
//! target, with the travelled vector (`deltaPos3`) accumulating the per-frame
//! step; the effect stops emitting once the target is within 2.5 units.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct GrimTooth {
    lifetime: i32,
}

impl GrimTooth {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for GrimTooth {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let angles = Angles::shared();

        if frame == 0 {
            runtime.play_sound("effect\\EF_FrostDiver.wav");

            // Face the target, then build the 2-unit forward step.
            let delta_x = runtime.target_offset[0] - runtime.position[0];
            let delta_z = runtime.target_offset[2] - runtime.position[2];

            let mut face_angle = if delta_x != 0.0 || delta_z != 0.0 {
                ((delta_x as f64).atan2((-delta_z) as f64) as f32 / 3.141592f32) * 180.0
            } else {
                0.0
            };

            if face_angle >= 360.0 {
                face_angle -= 360.0;
            }

            runtime.render_angle = face_angle;

            let mut rotation = super::super::math::Matrix::identity();
            rotation.make_y_rotation(angles.radian(-face_angle as i32));
            let step = matrix_mult([0.0, 0.0, 2.0], &rotation.values);
            runtime.delta_pos2 = step;
            runtime.delta_pos3[0] -= step[0];
            runtime.delta_pos3[1] -= step[1];
            runtime.delta_pos3[2] -= step[2];
        }

        if runtime.flag != 0 {
            return true;
        }

        if frame % 3 == 0 {
            // `CalcDist(deltaPos.x, deltaPos.z) <= 2.5` stops the burst.
            let distance = ((runtime.target_offset[2] - runtime.position[2]).powi(2)
                + (runtime.target_offset[0] - runtime.position[0]).powi(2))
            .sqrt();

            if distance <= 2.5 {
                runtime.flag = 1;
            }

            // RNG order: size, longitude, latitude (the angle/rand draws at the
            // top of the block are discarded by the original).
            let size = (runtime.rng.clamped_random(40) + 60) as f32 / 100.0;
            let longitude = runtime.rng.clamped_random(360) as f32;
            let latitude = (runtime.rng.clamped_random(30) + 75) as f32;

            let travelled = runtime.delta_pos3;
            let burst = runtime.launch(PrimitiveId::QuadHorn3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(burst);

            state.pattern |= 1;
            state.duration = 40;
            state.delta_pos2 = [travelled[0], 20.0 + travelled[1], travelled[2]];
            let burst_delta = state.delta_pos2;
            state.size = size;
            state.height_size = 10.0;
            state.longitude = longitude;
            state.latitude = latitude;
            state.transform.make_x_rotation(angles.radian(latitude as i32));
            state.transform.append_y_rotation(angles.radian(longitude as i32));
            state.rot_transform = state.transform;
            state.speed = 3.0;
            let decay = -(state.speed / state.duration as f32);
            state.accel = decay + decay;
            state.ch_cnt = 6;
            state.fade_out_cnt = state.duration - 10;
            state.texture = Some("effect\\stone.bmp");

            runtime.delta_pos2 = burst_delta;
            runtime.delta_pos3[0] -= burst_delta[0];
            runtime.delta_pos3[1] -= burst_delta[1];
            runtime.delta_pos3[2] -= burst_delta[2];
        } else {
            runtime.delta_pos3[0] -= runtime.delta_pos2[0];
            runtime.delta_pos3[1] -= runtime.delta_pos2[1];
            runtime.delta_pos3[2] -= runtime.delta_pos2[2];
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn grim_tooth_bursts_along_the_path() {
        let mut effect = GrimTooth::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 521);
        runtime.target_offset = [0.0, 0.0, 30.0];

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_FrostDiver.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let burst = &runtime.primitives[0].state;
        assert_eq!(burst.count, PrimitiveId::QuadHorn3d);
        assert_eq!(burst.pattern & 1, 1);
        assert_eq!(burst.duration, 40);
        // Spawned at 3.0, one motion step applied the deceleration.
        assert!((2.8..=3.0).contains(&burst.speed));
        assert!((burst.accel + 3.0 / 40.0 * 2.0).abs() < 1e-5);
        assert_eq!(burst.ch_cnt, 6);
        assert_eq!(burst.fade_out_cnt, 30);
        assert_eq!(burst.height_size, 10.0);
        assert_eq!(burst.texture, Some("effect\\stone.bmp"));
        assert!(burst.latitude >= 75.0 && burst.latitude <= 104.0);
        assert!((0.6..1.0).contains(&burst.size));
    }

    #[test]
    fn bursts_stop_at_the_target() {
        let mut effect = GrimTooth::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 521);
        runtime.target_offset = [0.0, 0.0, 1.0];

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.flag, 1, "target within 2.5 stops further bursts");
    }
}
