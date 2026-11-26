//! `CRagEffect::DecAgility` - `RagEffect.cpp:15222-15279`, effect id
//! `EF_DECAGILITY`, lifetime 100 frames.
//!
//! The slow banner (`slow.bmp`, drifting down at 180 degrees) plus an
//! `ac_center2.tga` mote every even frame, accelerating outward.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct DecAgility {
    spawned: bool,
}

impl PortedEffect for DecAgility {
    fn lifetime(&self) -> i32 {
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let angles = Angles::shared();

        if runtime.frame == 0 {
            self.spawned = true;
            runtime.play_sound("effect\\EF_DecAgility.wav");

            let duration = runtime.lifetime;
            let banner = runtime.launch(PrimitiveId::Texture2d, [0.0, 0.0, 0.0]);
            {
                let state = runtime.primitive_mut(banner);
                state.duration = duration;
                state.delta_pos2[1] -= 80.0;
                state.longitude = 180.0;
                state.speed = 1.0;
                state.accel = -(state.speed / state.duration as f32);
                state.width_size = 40.0;
                state.height_size = 10.0;
                state.alpha = 0.0;
                state.max_alpha = 200.0;
                state.alpha_speed = state.max_alpha / 15.0;
                state.fade_out_cnt = state.duration - 15;
                state.texture = Some("effect\\slow.bmp");
            }
        }

        if (runtime.frame & 1) != 0 {
            return true;
        }

        // RNG order: rotation, forward distance, width.
        let rotation = runtime.rng.clamped_random(360);
        let forward = (runtime.rng.clamped_random(7) + 2) as f32;
        let width_size = (runtime.rng.clamped_random(60) + 30) as f32 / 10.0;

        let mote = runtime.launch(PrimitiveId::CrossTexture3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(mote);

        state.duration = 50;
        state.transform.make_y_rotation(angles.radian(rotation));

        let offset = matrix_mult([0.0, 0.0, forward], &state.transform.values);
        state.delta_pos2 = [offset[0], offset[1] - 20.0, offset[2]];

        state.transform.make_x_rotation(angles.radian(-90));
        state.accel = 0.015;
        state.width_size = width_size;
        state.height_size = 0.18;
        state.alpha = 0.0;
        state.max_alpha = 200.0;
        state.alpha_speed = state.max_alpha / 20.0;
        state.fade_out_cnt = state.duration - 20;
        state.texture = Some("effect\\ac_center2.tga");

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn frame_zero_spawns_the_slow_banner_and_a_mote() {
        let mut effect = DecAgility::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 101);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_DecAgility.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 2);

        let banner = &runtime.primitives[0].state;
        assert_eq!(banner.duration, 100);
        assert_eq!(banner.longitude, 180.0);
        assert_eq!(banner.width_size, 40.0);
        assert_eq!(banner.height_size, 10.0);
        assert!((banner.alpha_speed - 200.0 / 15.0).abs() < 1e-4);
        assert_eq!(banner.fade_out_cnt, 85);
        assert_eq!(banner.texture, Some("effect\\slow.bmp"));
        assert_eq!(banner.delta_pos2[1], -80.0);
        // Heading 180 deg: cos(180) = -1, so the banner drifts down (+y).
        assert!(banner.delta_pos[1] > 0.0, "drifting down");

        let mote = &runtime.primitives[1].state;
        assert_eq!(mote.count, PrimitiveId::CrossTexture3d);
        assert_eq!(mote.duration, 50);
        assert!((mote.accel - 0.015).abs() < 1e-6);
        assert!((3.0..9.0).contains(&mote.width_size));
        assert_eq!(mote.height_size, 0.18);
        assert_eq!(mote.fade_out_cnt, 30);
        assert_eq!(mote.texture, Some("effect\\ac_center2.tga"));
    }

    #[test]
    fn motes_only_spawn_on_even_frames() {
        let mut effect = DecAgility::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 101);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 3);
    }
}
