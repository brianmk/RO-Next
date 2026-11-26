//! `CRagEffect::StoneSpike` - `RagEffect.cpp:13342-13385`.
//!
//! Heaven's Drive: a 5×5 grid of stone spikes with a quake, the effect lifetime
//! extended by 50 frames and every spike carrying its own random size, height,
//! orientation and duration.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct StoneSpike {
    lifetime: i32,
}

impl StoneSpike {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for StoneSpike {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();
        let mut lifetime = runtime.lifetime;
        let position = runtime.position;

        runtime.play_sound("effect\\wizard_earthspike.wav");
        runtime.quake();

        // `state.duration += 50` before the spikes read it.
        lifetime += 50;

        for z in (-10..15).step_by(5) {
            for x in (-10..15).step_by(5) {
                // RNG order: duration, heading (discarded), height, size,
                // longitude, latitude.
                let duration = runtime.rng.clamped_random(30) + lifetime - 10;
                let _heading = runtime.rng.clamped_random(360);
                let height_rand = runtime.rng.clamped_random(60);
                let size = (runtime.rng.clamped_random(50) + 300) as f32 / 100.0;
                let longitude = runtime.rng.clamped_random(360) as f32;
                let latitude = (runtime.rng.clamped_random(20) + 80) as f32;

                let spike = runtime.launch(PrimitiveId::QuadHorn3d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(spike);

                state.duration = duration;
                state.pattern = 1 | 2;
                state.delta_pos2 = [x as f32, 10.0, z as f32];
                state.size = size;
                state.height_size = (height_rand + 1) as f32 / 10.0 + 9.0;
                state.longitude = longitude;
                state.latitude = latitude;
                state.transform.make_x_rotation(angles.radian(latitude as i32));
                state.transform.append_y_rotation(angles.radian(longitude as i32));
                state.rot_transform = state.transform;
                state.change_speed3d[1] = -1.0;
                state.change_accel3d[1] = -0.01;
                state.speed = 1.0;
                state.accel = 0.01;
                state.change_point = 11;
                state.change_speed = -1.2;
                state.change_accel = 0.0;
                state.ch_cnt = 14;
                state.texture = Some("effect\\stone.bmp");
                state.position = position;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn heavens_drive_spawns_the_grid() {
        let mut effect = StoneSpike::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 541);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\wizard_earthspike.wav".to_owned()]);
        assert_eq!(runtime.shakes, 1);
        assert_eq!(runtime.primitives.len(), 25, "5x5 grid");

        let first = &runtime.primitives[0].state;
        assert_eq!(first.count, PrimitiveId::QuadHorn3d);
        assert_eq!(first.pattern, 3, "pattern 1 | 2");
        assert_eq!(first.delta_pos2, [-10.0, 10.0, -10.0]);
        assert_eq!(first.change_point, 11);
        assert!((first.change_speed + 1.2).abs() < 1e-6);
        assert_eq!(first.ch_cnt, 14);
        assert!((first.speed - 1.0).abs() < 0.02);
        assert!(first.duration >= 250 + 50 - 10);
        assert!((9.0..=15.0).contains(&first.height_size));
        assert!((3.0..3.5).contains(&first.size));
        assert_eq!(first.texture, Some("effect\\stone.bmp"));

        // The last spike sits at the far corner of the grid.
        let last = &runtime.primitives[24].state;
        assert_eq!(last.delta_pos2, [10.0, 10.0, 10.0]);
    }
}
