//! `CRagEffect::TexturedSkillAura(type)` - `RagEffect.cpp:15281-15370`.
//!
//! The small rising texture aura of Detect / Analyse-style skills. One
//! `PP_3DTEXTURE` primitive on frame 0 with `fashasha.tga`, type-specific
//! sound/colour and a `changePoint` size burst.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TexturedSkillAura {
    aura_type: i32,
    lifetime: i32,
}

impl TexturedSkillAura {
    pub fn new(aura_type: i32, lifetime: i32) -> Self {
        Self { aura_type, lifetime }
    }

    fn sound(&self) -> Option<&'static str> {
        match self.aura_type {
            0 => Some("effect\\hunter_detecting.wav"),
            2 => Some("effect\\priest_benedictio.wav"),
            4 => Some("effect\\cru_grand cross.wav"),
            5 => Some("effect\\ANALYZE.wav"),
            _ => None,
        }
    }

    fn color(&self) -> Option<(i32, i32, i32)> {
        match self.aura_type {
            4 => Some((0xFF, 0x8C, 0x8C)),
            5 => Some((0xD2, 0xFA, 0xD2)),
            6 => Some((10, 10, 10)),
            7 => Some((0xFA, 0xD2, 0x8C)),
            8 => Some((0x8C, 0xFF, 0x8C)),
            _ => None,
        }
    }
}

impl PortedEffect for TexturedSkillAura {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        if let Some(sound) = self.sound() {
            runtime.play_sound(sound);
        }

        let aura = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(aura);

        state.duration = 57;
        state.width_speed = 1.5;
        state.height_speed = 1.5;
        state.width_accel = state.width_speed / 14.0;
        state.height_accel = state.height_speed / 14.0;
        state.delta_pos2[1] = -7.0;
        state.speed3d[1] = -0.5;
        state.roll = 90.0;

        if self.aura_type == 3 || self.aura_type == 6 {
            state.change_point = 10;
            state.fade_out_cnt = 30;
        } else {
            state.change_point = 17;
            state.fade_out_cnt = 40;
        }

        if let Some((red, green, blue)) = self.color() {
            state.red = red;
            state.green = green;
            state.blue = blue;
        }

        state.change_width_speed = 0.1;
        state.change_height_speed = 0.1;
        state.change_width_accel = state.change_width_speed / 30.0;
        state.change_height_accel = state.change_height_speed / 30.0;
        // The original assigns deltaPos2.y twice; the second assignment wins.
        state.delta_pos2[1] = -5.0;
        state.texture = Some("effect\\fashasha.tga");

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn detecting_uses_the_default_burst_window() {
        let mut effect = TexturedSkillAura::new(0, 250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 201);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\hunter_detecting.wav".to_owned()]);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Texture3d);
        assert_eq!(state.duration, 57);
        assert_eq!(state.change_point, 17);
        assert_eq!(state.fade_out_cnt, 40);
        assert_eq!(state.roll, 90.0);
        assert_eq!(state.delta_pos2[1], -5.0);
        assert_eq!(state.texture, Some("effect\\fashasha.tga"));
        assert!((state.width_accel - 1.5 / 14.0).abs() < 1e-6);
        assert!((state.change_width_accel - 0.1 / 30.0).abs() < 1e-6);
    }

    #[test]
    fn type_three_changes_the_burst_window() {
        let mut effect = TexturedSkillAura::new(3, 250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 201);

        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.change_point, 10);
        assert_eq!(state.fade_out_cnt, 30);
        assert!(runtime.sounds.is_empty(), "type 3 has no sound");
    }

    #[test]
    fn type_four_tints_the_aura() {
        let mut effect = TexturedSkillAura::new(4, 250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 201);

        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!((state.red, state.green, state.blue), (0xFF, 0x8C, 0x8C));
        assert_eq!(runtime.sounds, vec!["effect\\cru_grand cross.wav".to_owned()]);
    }
}
