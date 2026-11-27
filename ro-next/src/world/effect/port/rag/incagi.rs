//! `CRagEffect::IncAgility` / `CRagEffect::IncAgiDex` -
//! `RagEffect.cpp:679-757`.
//!
//! `EF_INCAGILITY` (37) and `EF_INCAGIDEX` (43), both lifetime 100 frames.
//! Frame 0 spawns the rising buff glyph (`PP_2DTEXTURE`); every even frame adds
//! a `PP_3DCROSSTEXTURE` mote that drifts outward.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct IncAgility {
    /// `false` = Inc Agility (`agi_up.bmp`), `true` = Inc Agi-Dex
    /// (`dex_agi_up.bmp`).
    dex: bool,
}

impl IncAgility {
    pub fn new(dex: bool) -> Self {
        Self { dex }
    }
}

impl Default for IncAgility {
    fn default() -> Self {
        Self { dex: false }
    }
}

impl PortedEffect for IncAgility {
    fn lifetime(&self) -> i32 {
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let angles = Angles::shared();

        // `CopyVector(this->render.position, this->state.parent->render.position)`
        // (`RagEffect.cpp:685`), every frame, before the frame-0 launch.
        runtime.copy_parent_position();

        if runtime.frame == 0 {
            runtime.play_sound(if self.dex {
                "effect\\EF_IncAgiDex.wav"
            } else {
                "effect\\EF_IncAgility.wav"
            });

            let duration = runtime.lifetime;
            let glyph = runtime.launch(PrimitiveId::Texture2d, [0.0, 0.0, 0.0]);
            {
                let state = runtime.primitive_mut(glyph);
                state.duration = duration;
                state.longitude = 0.0;
                state.speed = 1.5;
                state.accel = -(state.speed / state.duration as f32) * 1.2;
                state.width_size = 40.0;
                state.height_size = 20.0;
                state.alpha = 0.0;
                state.max_alpha = 200.0;
                state.alpha_speed = state.max_alpha / 15.0;
                state.fade_out_cnt = state.duration - 15;
                state.texture = Some(if self.dex { "effect\\dex_agi_up.bmp" } else { "effect\\agi_up.bmp" });
                // `RagEffect.cpp:702` (IncAgility) / `:651` (IncAgiDex sets
                // `0`, the default). `99` keeps the glyph out of
                // `Render2DPrimitive`'s skill-cooldown branch, which keys on
                // `slots[0].life == 98` (`EffectPrimitiveRendering.cpp:7488`).
                state.slots[0].life = if self.dex { 0 } else { 99 };
            }
        }

        if (runtime.frame & 1) != 0 {
            return true;
        }

        // RNG order: rotation, forward distance, speed, width.
        let rotation = runtime.rng.clamped_random(360);
        let forward = (runtime.rng.clamped_random(7) + 2) as f32;
        let speed = (runtime.rng.clamped_random(50) + 20) as f32 / 100.0;
        let width_size = (runtime.rng.clamped_random(60) + 30) as f32 / 10.0;

        let mote = runtime.launch(PrimitiveId::CrossTexture3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(mote);

        state.duration = 50;
        state.transform.make_y_rotation(angles.radian(rotation));

        let offset = matrix_mult([0.0, 0.0, forward], &state.transform.values);
        state.delta_pos2 = offset;

        state.transform.make_x_rotation(angles.radian(90));
        state.speed = speed;
        state.width_size = width_size;
        state.height_size = 0.18;
        state.alpha = 0.0;
        state.max_alpha = 200.0;
        state.alpha_speed = state.max_alpha / 20.0;
        state.fade_out_cnt = state.duration - 20;
        state.texture = Some("effect\\ac_center2.tga");
        // `RagEffect.cpp:725` (IncAgility) / `:674` (IncAgiDex sets `0`).
        state.slots[0].life = if self.dex { 0 } else { 99 };

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn frame_zero_spawns_the_glyph_and_one_mote() {
        let mut effect = IncAgility::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 33);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_IncAgility.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 2);

        let glyph = &runtime.primitives[0].state;
        assert_eq!(glyph.count, PrimitiveId::Texture2d);
        assert_eq!(glyph.duration, 100);
        assert_eq!(glyph.width_size, 40.0);
        assert_eq!(glyph.height_size, 20.0);
        // The body sets `speed = 1.5` and `accel = -(speed / duration) * 1.2`;
        // the same step's `Move2DPrimitive` then applies the acceleration
        // once (`EffectPrimitiveMotion.cpp:7784`).
        assert!((glyph.accel - -(1.5 / 100.0) * 1.2).abs() < 1e-6);
        assert!((glyph.speed - (1.5 + glyph.accel)).abs() < 1e-6);
        assert!((glyph.alpha_speed - 200.0 / 15.0).abs() < 1e-4);
        assert_eq!(glyph.fade_out_cnt, 85);
        assert_eq!(glyph.texture, Some("effect\\agi_up.bmp"));
        // `RagEffect.cpp:702`: `slots[0].life = 99`.
        assert_eq!(glyph.slots[0].life, 99);

        let mote = &runtime.primitives[1].state;
        assert_eq!(mote.count, PrimitiveId::CrossTexture3d);
        assert_eq!(mote.duration, 50);
        assert!((0.2..0.7).contains(&mote.speed));
        assert!((3.0..9.0).contains(&mote.width_size));
        assert_eq!(mote.height_size, 0.18);
        assert_eq!(mote.texture, Some("effect\\ac_center2.tga"));
        assert_eq!(mote.slots[0].life, 99);
        // The mote is shot out along the Y-rotated `forward` vector; the
        // X rotation that follows does not change `deltaPos2`
        // (`RagEffect.cpp:744-748`).
        let distance =
            (mote.delta_pos2[0] * mote.delta_pos2[0] + mote.delta_pos2[1] * mote.delta_pos2[1] + mote.delta_pos2[2] * mote.delta_pos2[2])
                .sqrt();
        assert!((2.0..=9.0).contains(&distance), "forward distance = {distance}");
        assert!(mote.delta_pos2[1].abs() < 1e-4, "Y rotation keeps y at zero");
    }

    #[test]
    fn the_glyph_anchor_follows_the_parent_on_every_frame() {
        let mut effect = IncAgility::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 33);
        runtime.position = [1.0, 2.0, 3.0];
        runtime.parent_position = Some([4.0, 5.0, 6.0]);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.position, [4.0, 5.0, 6.0]);

        runtime.parent_position = Some([7.0, 8.0, 9.0]);
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.position, [7.0, 8.0, 9.0]);
    }

    #[test]
    fn motes_only_spawn_on_even_frames() {
        let mut effect = IncAgility::default();
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 33);

        // Frame 0 (glyph + mote), frame 1 (none), frame 2 (mote).
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2, "odd frame spawns nothing");

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 3, "even frame spawns a mote");
    }

    #[test]
    fn dex_variant_uses_its_own_sound_and_glyph() {
        let mut effect = IncAgility::new(true);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 33);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_IncAgiDex.wav".to_owned()]);
        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\dex_agi_up.bmp"));
        // `IncAgiDex` leaves `slots[0].life` at zero (`RagEffect.cpp:651`).
        assert_eq!(runtime.primitives[0].state.slots[0].life, 0);
    }
}
