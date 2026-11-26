//! `CRagEffect::IncAgilityCenter(kind)` used by `EF_AGIUP` -
//! `RagEffect.cpp:16534-16582`, dispatch at `:23334`.
//!
//! One sound on frame 0, then an `ac_center2.tga` mote every even frame with a
//! low `maxAlpha` (60) so the buff reads as a subtle aura.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct AgilityUp {
    /// `kind` from `IncAgilityCenter`; 0 for `EF_AGIUP`.
    kind: i32,
}

impl AgilityUp {
    pub fn new(kind: i32) -> Self {
        Self { kind }
    }
}

impl Default for AgilityUp {
    fn default() -> Self {
        Self { kind: 0 }
    }
}

impl PortedEffect for AgilityUp {
    fn lifetime(&self) -> i32 {
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.play_sound("effect\\EF_IncAgility.wav");
        }

        if (runtime.frame & 1) != 0 {
            return true;
        }

        let angles = Angles::shared();

        // RNG order: rotation, forward distance, speed, width, height.
        let rotation = runtime.rng.clamped_random(360);
        let forward = (runtime.rng.clamped_random(7) + 2) as f32;
        let (speed, width_size, height_size) = if self.kind >= 2 {
            (
                (runtime.rng.clamped_random(50) + 40) as f32 / 100.0,
                (runtime.rng.clamped_random(100) + 40) as f32 / 10.0,
                runtime.rng.clamped_random(6) as f32 * 0.05 + 0.1,
            )
        } else {
            (
                (runtime.rng.clamped_random(50) + 20) as f32 / 100.0,
                (runtime.rng.clamped_random(100) + 20) as f32 / 10.0,
                runtime.rng.clamped_random(4) as f32 * 0.1 + 0.1,
            )
        };

        let mote = runtime.launch(PrimitiveId::CrossTexture3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(mote);

        state.duration = 50;
        state.transform.make_y_rotation(angles.radian(rotation));

        let offset = matrix_mult([0.0, 0.0, forward], &state.transform.values);
        state.delta_pos2 = offset;

        state.transform.make_x_rotation(angles.radian(90));
        state.speed = speed;
        state.width_size = width_size;
        state.height_size = height_size;
        state.alpha = 0.0;
        state.max_alpha = 60.0;
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
    fn frame_zero_plays_a_sound_and_spawns_a_soft_mote() {
        let mut effect = AgilityUp::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 141);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_IncAgility.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let mote = &runtime.primitives[0].state;
        assert_eq!(mote.count, PrimitiveId::CrossTexture3d);
        assert_eq!(mote.duration, 50);
        assert_eq!(mote.max_alpha, 60.0);
        assert_eq!(mote.alpha_speed, 3.0);
        assert_eq!(mote.fade_out_cnt, 30);
        assert!((0.2..0.7).contains(&mote.speed));
        assert!((2.0..12.0).contains(&mote.width_size));
        assert!((0.1..0.6).contains(&mote.height_size));
        assert_eq!(mote.texture, Some("effect\\ac_center2.tga"));
    }

    #[test]
    fn motes_only_on_even_frames() {
        let mut effect = AgilityUp::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 141);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2);
    }
}
