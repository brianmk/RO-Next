//! `CRagEffect::FrostDiver(char type)` - `RagEffect.cpp:456-529`, effect ids
//! `EF_FROSTDIVER` / `EF_FROSTDIVER2` / `EF_FROSTDIVER3` (variant passed as
//! `type`).
//!
//! The shard projectile: a `PP_3DQUADHORN` is launched every frame and pushed
//! along the caster's facing until the effect's travelled position reaches the
//! target (within 2.5 units) - then `flag` stops the stream.

use super::super::math::Matrix;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `EF_FROSTDIVER` lifetime in frames.
pub const LIFETIME: i32 = 150;

#[derive(Debug)]
pub struct FrostDiver {
    variant: i32,
}

impl FrostDiver {
    pub fn new(variant: i32) -> Self {
        Self { variant }
    }
}

impl Default for FrostDiver {
    fn default() -> Self {
        Self { variant: 0 }
    }
}

/// `RotatedForward(matrix, distance)` - `RagEffect.cpp:136-143`.
fn rotated_forward(matrix: &Matrix, distance: f32) -> [f32; 3] {
    let m = &matrix.values;

    [
        ((m[3] * 0.0 + m[0] * 0.0) + m[6] * distance) + m[9],
        ((m[4] * 0.0 + m[1] * 0.0) + m[7] * distance) + m[10],
        ((m[5] * 0.0 + m[2] * 0.0) + m[8] * distance) + m[11],
    ]
}

impl PortedEffect for FrostDiver {
    fn lifetime(&self) -> i32 {
        LIFETIME
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.face(runtime.target_offset[0], runtime.target_offset[2]);

            let mut rotation = Matrix::identity();
            rotation.make_y_rotation(-(runtime.render_angle as f32).to_radians());

            let reach = rotated_forward(&rotation, 5.0);
            runtime.delta_pos3[0] -= reach[0];
            runtime.delta_pos3[1] -= reach[1];
            runtime.delta_pos3[2] -= reach[2];

            let step = rotated_forward(&rotation, 2.0);
            runtime.target_offset = step;
        }

        if runtime.flag != 0 {
            return true;
        }

        let saved = runtime.position;
        runtime.position = [
            runtime.delta_pos3[0] + runtime.position[0],
            runtime.delta_pos3[1] + runtime.position[1],
            runtime.delta_pos3[2] + runtime.position[2],
        ];

        if runtime.calc_dist(runtime.target_offset[0], runtime.target_offset[2]) <= 2.5 {
            runtime.flag = 1;
        }

        runtime.position = saved;

        // RNG order from the original: angle, distance, size, heightSize,
        // longitude, latitude.
        let angle = runtime.rng.clamped_random(360);
        let distance = (runtime.rng.clamped_random(10) + 5) as f32 / 10.0;
        let size = (runtime.rng.clamped_random(40) + 60) as f32 / 100.0;
        let height_size = (runtime.rng.clamped_random(30) + 150) as f32 / 10.0;
        let longitude = runtime.rng.clamped_random(360) as f32;
        let latitude = (runtime.rng.clamped_random(30) + 75) as f32;

        let angles = super::super::angles::Angles::shared();
        let sine = angles.sine(angle);
        let cosine = angles.cosine(angle);

        let travelled = runtime.delta_pos3;
        let shard = runtime.launch(PrimitiveId::QuadHorn3d, [0.0, 0.0, 0.0]);
        {
            let state = runtime.primitive_mut(shard);
            state.pattern |= 1;
            state.duration = 40;
            state.delta_pos2 = [
                sine * distance + travelled[0],
                30.0 + travelled[1],
                -distance * cosine + travelled[2],
            ];
            state.size = size;
            state.height_size = height_size;
            state.longitude = longitude;
            state.latitude = latitude;
            state.transform.make_x_rotation(angles.radian(latitude as i32));
            state.transform.append_y_rotation(angles.radian(longitude as i32));
            state.rot_transform = state.transform;
            state.speed = 3.0;
            let decay = -(state.speed / state.duration as f32);
            state.accel = decay + decay;
            state.ch_cnt = 20;
            state.fade_out_cnt = state.duration - 10;
            state.texture = Some("effect\\ice.tga");
        }

        let target = runtime.target_offset;
        runtime.delta_pos3[0] -= target[0];
        runtime.delta_pos3[1] -= target[1];
        runtime.delta_pos3[2] -= target[2];

        let _ = self.variant;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    const EPSILON: f32 = 1e-4;

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(LIFETIME, [0.0, 0.0], 13);
        runtime.target_offset = [10.0, 0.0, 0.0];
        runtime
    }

    #[test]
    fn launches_a_shard_every_frame_until_the_target() {
        let mut effect = FrostDiver::default();
        let mut runtime = runtime();

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::QuadHorn3d);
        assert_eq!(state.duration, 40);
        assert_eq!(state.pattern & 1, 1);
        assert_eq!(state.ch_cnt, 20);
        assert_eq!(state.fade_out_cnt, 30);
        // Spawned at 3.0, already integrated one frame of acceleration.
        let accel = -(3.0f32 / 40.0) * 2.0;
        assert!((state.speed - (3.0 + accel)).abs() < EPSILON);
        assert!((state.accel - accel).abs() < EPSILON);
        // (ClampedRandom(30) + 150) / 10 -> 15.0..17.9
        assert!((15.0..18.0).contains(&state.height_size));
        assert!((0.6..1.0).contains(&state.size));
    }

    #[test]
    fn facing_points_at_the_target_offset() {
        let mut effect = FrostDiver::default();
        let mut runtime = runtime();

        step(&mut effect, &mut runtime);

        // Target is +x from the origin -> facing ~90 degrees (atan2(10, -0)).
        assert!((runtime.render_angle - 90.0).abs() < 1.0);
    }

    #[test]
    fn stops_launching_once_the_flag_is_set() {
        let mut effect = FrostDiver::default();
        let mut runtime = runtime();

        step(&mut effect, &mut runtime);
        let first = runtime.primitives.len();

        runtime.flag = 1;
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), first);
    }
}
