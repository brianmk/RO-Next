//! `CRagEffect::IceWall` - `RagEffect.cpp:587-628`.
//!
//! Three stone/ice shards (`PP_3DQUADHORN`) planted around the cell; the effect
//! overrides its duration to 9999 frames, like the original.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct IceWall {
    spawned: bool,
}

impl PortedEffect for IceWall {
    fn lifetime(&self) -> i32 {
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        self.spawned = true;
        runtime.play_sound("effect\\wizard_icewall.wav");
        runtime.lifetime = 9999;

        let lifetime = runtime.lifetime;
        let angles = Angles::shared();

        for offset in [-2i32, 0, 2] {
            let angle = runtime.rng.clamped_random(360);
            let size = runtime.rng.clamped_random(40) as f32 / 100.0 + 2.5;
            let height_size = (runtime.rng.clamped_random(100) + 300) as f32 / 10.0;
            let longitude = runtime.rng.clamped_random(360) as f32;
            let latitude = (runtime.rng.clamped_random(6) + 87) as f32;

            let sine = angles.sine(angle);
            let cosine = angles.cosine(angle);

            let shard = runtime.launch(PrimitiveId::QuadHorn3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(shard);

            state.duration = lifetime;
            state.pattern |= 1;
            state.delta_pos2 = [sine * 0.0, 30.0, -(offset as f32) * cosine];
            state.size = size;
            state.height_size = height_size;
            state.longitude = longitude;
            state.latitude = latitude;
            state.ch_cnt = 20;
            state.alpha = 200.0;
            state.transform.make_x_rotation(angles.radian(latitude as i32));
            state.transform.append_y_rotation(angles.radian(longitude as i32));
            state.rot_transform = state.transform;
            state.speed = 3.0;
            let decay = -(state.speed / 40.0);
            state.accel = decay + decay;
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
    fn spawns_three_shards_and_extends_lifetime() {
        let mut effect = IceWall::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 17);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\wizard_icewall.wav".to_owned()]);
        assert_eq!(runtime.lifetime, 9999);
        assert_eq!(runtime.primitives.len(), 3);

        for shard in &runtime.primitives {
            let state = &shard.state;
            assert_eq!(state.count, PrimitiveId::QuadHorn3d);
            assert_eq!(state.duration, 9999);
            assert_eq!(state.pattern & 1, 1);
            assert_eq!(state.ch_cnt, 20);
            assert_eq!(state.alpha, 200.0);
            assert_eq!(state.delta_pos2[1], 30.0);
            assert!((2.5..=2.9).contains(&state.size));
            assert!((30.0..40.0).contains(&state.height_size));
        }

        // Each shard gets its own random angle, so only the centre shard
        // (offset 0) is guaranteed to sit on the anchor.
        let z: Vec<f32> = runtime.primitives.iter().map(|p| p.state.delta_pos2[2]).collect();
        assert_eq!(z[1], 0.0);
        assert!(z.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn survives_past_the_table_lifetime() {
        let mut effect = IceWall::default();
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 17);

        for _ in 0..300 {
            assert!(step(&mut effect, &mut runtime));
        }
    }
}
