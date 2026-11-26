//! `CRagEffect::StormGust` - `RagEffect.cpp:15176-15215`.
//!
//! Storm Gust's ice fragments: two `PP_QUADHORN3D` (9) pieces every 29 frames
//! between frames 31 and 150, drifting sideways with the official
//! `X(100) → Y(degree)` orientation.
//!
//! The `.str` cloud overlay is driven by `ProcessEZ2STR(0, -80, 0)`; the port
//! calls `process_str` with the same offset so the overlay follows the effect
//! once the caller opens its clip.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct StormGust {
    opened: bool,
}

impl PortedEffect for StormGust {
    fn lifetime(&self) -> i32 {
        9999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 0 {
            self.opened = true;
            runtime.play_sound("effect\\wizard_stormgust.wav");
        }

        if frame % 29 == 0 && frame > 30 && frame <= 150 {
            let angles = Angles::shared();

            for _ in 0..2 {
                // RNG order: degree, radius, angle, size.
                let degree = runtime.rng.clamped_random(360) as f32;
                let radius = (runtime.rng.clamped_random(25) + 3) as f32;

                let angle = runtime.rng.clamped_random(360) as f32;
                let sine = angles.raw_sine(angle as usize);
                let cosine = angles.raw_cosine(angle as usize);

                let size = (runtime.rng.clamped_random(100) + 400) as f32 / 100.0;

                let fragment = runtime.launch(PrimitiveId::QuadHorn3d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(fragment);

                state.duration = 215 - frame;
                state.pattern |= 1;
                state.delta_pos2 = [radius * sine, 20.0, -radius * cosine];
                state.size = size;
                state.height_size = 20.0;
                state.longitude = degree;
                state.latitude = 100.0;

                state.transform.make_x_rotation(angles.radian(100));
                state.transform.append_y_rotation(angles.radian(degree as i32));
                state.rot_transform = state.transform;

                state.speed = 1.5;
                state.accel = 0.0;
                state.alpha = 250.0;
                state.ch_cnt = 3;
                state.fade_out_cnt = state.duration - 10;
                state.texture = Some("effect\\ice.tga");
            }
        }

        // `ProcessEZ2STR(0, -80, 0)` advances the cloud overlay; the offset is
        // applied by the clip when it is opened.
        runtime.process_str();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn fragments_start_after_frame_thirty() {
        let mut effect = StormGust::default();
        let mut runtime = EffectRuntime::new(9999, [0.0, 0.0], 271);

        while runtime.frame < 57 {
            step(&mut effect, &mut runtime);
        }
        assert!(runtime.primitives.is_empty(), "frame 57 is not a burst frame");

        // The first burst is at frame 58 (58 % 29 == 0, frame > 30).
        while runtime.frame <= 58 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), 2);
        assert_eq!(runtime.sounds, vec!["effect\\wizard_stormgust.wav".to_owned()]);

        let fragment = &runtime.primitives[0].state;
        assert_eq!(fragment.count, PrimitiveId::QuadHorn3d);
        assert_eq!(fragment.duration, 215 - 58);
        assert_eq!(fragment.pattern & 1, 1);
        assert_eq!(fragment.alpha, 250.0);
        assert_eq!(fragment.ch_cnt, 3);
        assert_eq!(fragment.texture, Some("effect\\ice.tga"));
        assert!((4.0..5.5).contains(&fragment.size));
        assert_eq!(fragment.height_size, 20.0);
        assert_eq!(fragment.latitude, 100.0);
        assert!((fragment.delta_pos2[1] - 20.0).abs() < 1e-6);
    }

    #[test]
    fn bursts_stop_after_frame_one_hundred_fifty() {
        let mut effect = StormGust::default();
        let mut runtime = EffectRuntime::new(9999, [0.0, 0.0], 271);

        while runtime.frame < 150 {
            step(&mut effect, &mut runtime);
        }
        let at_150 = runtime.primitives.len();
        while runtime.frame < 200 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), at_150);
    }
}
