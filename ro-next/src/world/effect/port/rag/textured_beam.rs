//! `CRagEffect::TexturedBeam` - `RagEffect.cpp:3043-3079`.
//!
//! Spawns one prim-154 blade-fin quad strip at frame 0, anchored to the caster
//! and aimed from the effect's `deltaPos` toward the target. The four slots are
//! cleared and only slot 0 is armed: `distance = 100`, `height[0] = 50`,
//! `maxHeight = 5`, `height[2] = 80`, and `flag1[0] = flag` selects the colour
//! pair in `RenderBladeFins`.
//!
//! The body is registered on its own for `EF_CHEMICAL_V3` (which calls only
//! `TexturedBeam(2, ...)`) and is reused by [`super::ChemicalLine`].

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `CRagEffect::TexturedBeam(char flag, const char* name)`.
#[derive(Debug)]
pub struct TexturedBeam {
    flag: i8,
    name: &'static str,
    lifetime: i32,
}

impl TexturedBeam {
    pub fn new(flag: i8, name: &'static str, lifetime: i32) -> Self {
        Self { flag, name, lifetime }
    }
}

impl PortedEffect for TexturedBeam {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        // `Get_Angle(deltaPos.x - render.position.x, deltaPos.z - render.position.z)`.
        let rise_angle = runtime.get_angle(
            runtime.target_offset[0] - runtime.position[0],
            runtime.target_offset[2] - runtime.position[2],
        );

        let duration = runtime.lifetime;
        let index = runtime.launch(PrimitiveId::FlashFade, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = duration;
        state.texture = Some(self.name);

        for slot in &mut state.slots {
            slot.life = 0;
        }

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.process = 0;
        slot.distance = 100.0;
        slot.height[0] = 50.0;
        slot.max_height = 5.0;
        slot.height[2] = 80.0;
        slot.rise_angle = rise_angle;
        slot.alpha_b = 0;
        slot.flag1[0] = self.flag;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::EffectRuntime;

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);
        // Target 10 units along +z. `Get_Angle(0, 10)` lands on table index 91,
        // not 90: `g_cosTable[90]` is `+3.3e-7` because the table uses
        // `3.141592f`, so the `normalX >= cos` test only holds one step later.
        runtime.target_offset = [0.0, 0.0, 10.0];
        runtime
    }

    #[test]
    fn frame_zero_arms_the_one_blade_fin_slot() {
        let mut effect = TexturedBeam::new(1, "effect\\line3.tga", 300);
        let mut runtime = runtime();

        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::FlashFade);
        assert_eq!(state.count.id(), 154);
        assert_eq!(state.duration, 300);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.texture, Some("effect\\line3.tga"));
        assert_eq!(state.total_texture, 1);
        assert_eq!(state.anim_speed, 4, "the constructor default, not written by the body");

        assert_eq!(state.slots[0].life, 1);

        for slot in &state.slots[1..] {
            assert_eq!(slot.life, 0);
        }

        let slot = &state.slots[0];
        assert_eq!(slot.process, 0);
        assert_eq!(slot.distance, 100.0);
        assert_eq!(slot.height[0], 50.0);
        assert_eq!(slot.max_height, 5.0);
        assert_eq!(slot.height[2], 80.0);
        assert_eq!(slot.rise_angle, 91);
        assert_eq!(slot.alpha_b, 0);
        assert_eq!(slot.flag1[0], 1);
    }

    #[test]
    fn flag_goes_into_slot_zero_flag1() {
        for flag in [0i8, 1, 2] {
            let mut effect = TexturedBeam::new(flag, "effect\\line3.tga", 300);
            let mut runtime = runtime();

            effect.on_process(&mut runtime);

            assert_eq!(runtime.primitives[0].state.slots[0].flag1[0], flag);
        }
    }

    #[test]
    fn later_frames_do_not_spawn_again() {
        let mut effect = TexturedBeam::new(2, "effect\\line3.tga", 300);
        let mut runtime = runtime();

        effect.on_process(&mut runtime);
        runtime.frame = 1;
        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
    }
}
