//! `CRagEffect::FrostDiver2` - `RagEffect.cpp:414-452`, effect id
//! `EF_FROSTDIVER2`.
//!
//! Eight ice shards (`PP_3DQUADHORN`) fanned around the target cell, launched
//! once on frame 0.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct FrostDiver2 {
    spawned: bool,
}

impl PortedEffect for FrostDiver2 {
    fn lifetime(&self) -> i32 {
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        self.spawned = true;
        runtime.play_sound("effect\\EF_FrostDiver2.wav");

        let angles = Angles::shared();

        for _ in 0..8 {
            // RNG order: angle, length, size, heightSize, longitude, latitude.
            let angle = runtime.rng.clamped_random(360);
            let length = (runtime.rng.clamped_random(4) + 1) as f32 / 10.0;
            let size = (runtime.rng.clamped_random(250) + 100) as f32 / 100.0;
            let height_size = (runtime.rng.clamped_random(100) + 200) as f32 / 10.0;
            let longitude = runtime.rng.clamped_random(360) as f32;
            let latitude = (runtime.rng.clamped_random(20) + 80) as f32;

            let sine = angles.sine(angle);
            let cosine = angles.cosine(angle);

            let shard = runtime.launch(PrimitiveId::QuadHorn3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(shard);

            state.duration = 40;
            state.pattern |= 1;
            state.delta_pos2 = [sine * length, 30.0, -length * cosine];
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
            state.alpha = 200.0;
            state.fade_out_cnt = state.duration - 10;
            state.texture = Some("effect\\ice.tga");
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn spawns_eight_ice_shards() {
        let mut effect = FrostDiver2::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 71);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_FrostDiver2.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 8);

        for shard in &runtime.primitives {
            let state = &shard.state;
            assert_eq!(state.count, PrimitiveId::QuadHorn3d);
            assert_eq!(state.duration, 40);
            assert_eq!(state.pattern & 1, 1);
            assert_eq!(state.delta_pos2[1], 30.0);
            assert!((1.0..3.5).contains(&state.size));
            assert!((20.0..30.0).contains(&state.height_size));
            assert_eq!(state.ch_cnt, 20);
            assert_eq!(state.alpha, 200.0);
            assert_eq!(state.fade_out_cnt, 30);
        }
    }
}
