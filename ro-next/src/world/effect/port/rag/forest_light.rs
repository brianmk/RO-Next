//! `EF_FORESTLIGHT6` dispatch block - `RagEffect.cpp:19940-19950`, built from
//! `CRagEffect::ForestLight` (`:14597-14664`), `TexturedHalo` and
//! `WhiteRingRipple`.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};
use super::ring_ripple::white_ring_ripple;
use super::textured_halo::TexturedHalo;

/// `CRagEffect::ForestLight(name, flag)`.
pub(crate) fn forest_light(runtime: &mut EffectRuntime, name: &'static str, flag: i32) {
    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;
    let position = runtime.position;

    let light = runtime.launch(PrimitiveId::ForestLight, [0.0, 0.0, 0.0]);
    let state = runtime.primitive_mut(light);

    state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
    state.duration = lifetime;
    state.texture = Some(name);

    for slot in &mut state.slots {
        slot.life = 0;
    }

    for index in 0..4usize {
        let mut delta = position;
        delta[1] -= if flag == 10 { 60.0 } else { 300.0 };
        delta[0] -= if flag == 10 { 15.0 } else { 70.0 };
        delta[2] -= if flag == 10 { 15.0 } else { 70.0 };

        let max_height = if index == 3 {
            if flag == 1 || flag == 11 {
                4.0
            } else if flag == 2 || flag == 12 || flag == 13 {
                1.0
            } else {
                2.0
            }
        } else if flag == 1 || flag == 11 {
            index as f32 * 2.0 + 4.0
        } else if flag == 2 || flag == 12 || flag == 13 {
            index as f32 * 0.5 + 1.0
        } else {
            index as f32 + 2.0
        };

        let slot = &mut state.slots[index];

        slot.life = 1;
        slot.process = if index == 3 { 180 } else { 0 };
        slot.alpha_b = if flag == 2 || flag == 12 || flag == 13 {
            30
        } else if flag == 10 {
            25
        } else if flag == 11 {
            0
        } else {
            40
        };
        slot.distance = 0.0;
        slot.max_height = max_height;
        slot.height[0] = max_height;
        slot.rot_start = (index as i16) * 25;
        slot.vec_t_now = position;
        slot.vec_t_pre = delta;
        slot.flag1[0] = flag as i8;
    }
}

#[derive(Debug)]
pub struct ForestLight6 {
    lifetime: i32,
}

impl ForestLight6 {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for ForestLight6 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 1 {
            runtime.play_sound("effect\\dimension.wav");
        }

        forest_light(runtime, "effect\\cloud11.tga", 13);

        if frame == 0 {
            TexturedHalo::new("effect\\구체15.bmp", 4, self.lifetime).on_process(runtime);
        }

        white_ring_ripple(runtime, 0);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn forest_light_builds_its_four_slots() {
        let mut effect = ForestLight6::new(29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 461);
        runtime.position = [10.0, 20.0, 30.0];

        step(&mut effect, &mut runtime);

        // Forest light + halo + ripple.
        assert_eq!(runtime.primitives.len(), 3);

        let light = &runtime.primitives[0].state;
        assert_eq!(light.count, PrimitiveId::ForestLight);
        assert_eq!(light.duration, 29999);
        assert_eq!(light.texture, Some("effect\\cloud11.tga"));
        assert!(light.slots.iter().all(|slot| slot.life == 1));
        assert!(light.slots.iter().all(|slot| slot.flag1[0] == 13));
        assert_eq!(light.slots[3].process, 180);
        assert_eq!(light.slots[0].max_height, 1.0);
        assert_eq!(light.slots[3].alpha_b, 30);
        // flag 13 -> deltaPos.y - 300 / x,z - 70
        assert_eq!(light.slots[0].vec_t_pre, [10.0 - 70.0, 20.0 - 300.0, 30.0 - 70.0]);
        assert_eq!(light.slots[0].rot_start, 0);
        assert_eq!(light.slots[2].rot_start, 50);

        assert_eq!(runtime.primitives[1].state.count, PrimitiveId::Halo);
        assert_eq!(runtime.primitives[2].state.count, PrimitiveId::RippleRings);
    }

    #[test]
    fn dimension_wave_lands_on_frame_one() {
        let mut effect = ForestLight6::new(29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 461);

        step(&mut effect, &mut runtime);
        assert!(runtime.sounds.is_empty());

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds, vec!["effect\\dimension.wav".to_owned()]);
    }
}
