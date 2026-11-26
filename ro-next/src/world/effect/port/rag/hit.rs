//! `CRagEffect::Hit2(int option)` - `RagEffect.cpp:807-859`.
//!
//! The generic eight-fragment impact (`EF_HIT2`, and the hit effect of most
//! physical skills via `Hit2(0)`/`Hit2(1)`). Eight `PP_2DTEXTURE` shards are
//! fired on a 45-degree ring with alternating `lens1.tga`/`lens2.tga`.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `g_depthAverageHalf` - `MapSkyMgr.cpp:27`.
const DEPTH_AVERAGE_HALF: f32 = 0.5;

#[derive(Debug)]
pub struct Hit2 {
    option: i32,
}

impl Hit2 {
    pub fn new(option: i32) -> Self {
        Self { option }
    }
}

impl Default for Hit2 {
    fn default() -> Self {
        Self { option: 0 }
    }
}

impl PortedEffect for Hit2 {
    fn lifetime(&self) -> i32 {
        50
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        runtime.play_sound("effect\\EF_hit2.wav");

        let angles = Angles::shared();

        let mut index = 0usize;
        let mut degree = 0.0f32;

        while degree < 360.0 {
            // RNG order: duration, heading offset, speed, width, height, spread.
            let duration = runtime.rng.clamped_random(20) + 10;
            let heading = (degree - 15.0) + runtime.rng.clamped_random(30) as f32;
            let speed = (runtime.rng.clamped_random(45) + 5) as f32 / 10.0;
            let width_size = (runtime.rng.clamped_random(15) + 5) as f32;
            let height_size = (runtime.rng.clamped_random(20) + 20) as f32;
            let spread = runtime.rng.clamped_random(5) as f32;

            let shard = runtime.launch(PrimitiveId::Texture2d, [0.0, 0.0, 0.0]);
            {
                let state = runtime.primitive_mut(shard);
                state.pattern = 1;
                state.duration = duration;
                state.roll = heading;
                state.longitude = heading;
                state.speed = speed;
                state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
                state.width_size = width_size;
                state.height_size = height_size;
                state.width_speed = -(state.width_size / state.duration as f32);
                state.height_speed = 1.5;
                state.height_accel = 0.25;

                state.delta_pos2[0] = angles.sine(degree as i32) * spread;
                state.delta_pos2[1] = angles.cosine(degree as i32) * -spread;
                state.delta_pos2[1] -= 20.0;

                if self.option != 0 {
                    state.red = 120;
                    state.green = 120;
                }

                state.alpha = 0.0;
                state.alpha_speed = state.max_alpha * 0.125;
                state.texture = Some(if index % 2 == 0 { "effect\\lens1.tga" } else { "effect\\lens2.tga" });
            }

            // The original alternates the texture by fragment index; the
            // texture assignment happens in the renderer.
            index += 1;
            let _ = index;

            degree += 45.0;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn spawns_eight_fragments() {
        let mut effect = Hit2::default();
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 51);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_hit2.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 8);

        for (index, fragment) in runtime.primitives.iter().enumerate() {
            let state = &fragment.state;
            assert_eq!(state.count, PrimitiveId::Texture2d);
            assert_eq!(state.pattern, 1);
            assert!((10..=29).contains(&state.duration), "duration {}", state.duration);
            assert!((0.5..=4.9).contains(&state.speed));
            assert!((5.0..=19.0).contains(&state.width_size));
            assert!((20.0..=39.0).contains(&state.height_size));
            // Spawned at heightSpeed 1.5 / heightAccel 0.25, one step applied.
            assert!((state.height_speed - 1.75).abs() < 1e-5);
            assert!((state.height_accel - 0.25).abs() < 1e-6);
            // Heading is degree - 15 + random(30).
            let expected_center = index as f32 * 45.0;
            assert!((state.longitude - expected_center).abs() <= 15.0);
        }
    }

    #[test]
    fn tinted_option_sets_rgb_to_120() {
        let mut effect = Hit2::new(1);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 51);

        step(&mut effect, &mut runtime);

        for fragment in &runtime.primitives {
            assert_eq!(fragment.state.red, 120);
            assert_eq!(fragment.state.green, 120);
        }
    }
}
