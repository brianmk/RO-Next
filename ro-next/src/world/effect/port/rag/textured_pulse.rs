//! `CRagEffect::TexturedPulse(const char* name)` - `RagEffect.cpp:2833-2858`.
//!
//! The shell of `EF_HALFSPHERE` (`RagEffect.cpp:23129-23131`): one
//! `PP_HALFSPHERE` (104) primitive on frame 0, textured with `white02.bmp`,
//! driven by slot 0 alone. The primitive's motion and renderer live in
//! [`super::super::primitive::half_sphere`] and
//! [`super::super::render_slots::render_half_sphere`].
//!
//! `TexturedPulse` starts by copying the caster's position into
//! `this->render.position` (`parent.position`); the port's
//! [`EffectRuntime::position`] is already that anchor and the primitive's
//! `FollowMaster` keeps it in sync, so there is no separate copy to make.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TexturedPulse {
    texture: &'static str,
    lifetime: i32,
}

impl TexturedPulse {
    pub fn new(texture: &'static str, lifetime: i32) -> Self {
        Self { texture, lifetime }
    }
}

impl PortedEffect for TexturedPulse {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `if (this->state.stateCnt != 0) return;`
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;

        let pulse = runtime.launch(PrimitiveId::HalfSphere, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(pulse);

        // `RSF_ALPHA | RSF_NO_DEPTH` (`RenderQueues.h:11,18`).
        state.render_flag = 0x1 | 0x200;
        state.duration = lifetime;
        state.total_texture = 1;
        state.texture = Some(self.texture);
        state.size = 0.0;

        state.slots[0].life = 1;
        state.slots[1].life = 0;
        state.slots[2].life = 0;
        state.slots[3].life = 0;

        state.slots[0].process = 0;
        state.slots[0].max_height = 8.0;
        state.slots[0].alpha_b = 0;
        state.slots[0].distance = 8.0;
        state.slots[0].full_display_angle = 180;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::render_slots;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    fn effect() -> TexturedPulse {
        TexturedPulse::new("effect\\white02.bmp", 200)
    }

    #[test]
    fn halfsphere_spawns_the_white_pulse_shell() {
        let mut effect = effect();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 17);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let pulse = &runtime.primitives[0].state;

        assert_eq!(pulse.count, PrimitiveId::HalfSphere);
        assert_eq!(pulse.count.id(), 104);
        assert_eq!(pulse.render_flag, 0x201);
        assert_eq!(pulse.duration, 200);
        assert_eq!(pulse.total_texture, 1);
        assert_eq!(pulse.texture, Some("effect\\white02.bmp"));
        assert_eq!(pulse.size, 0.0);

        assert_eq!(pulse.slots[0].life, 1);
        assert_eq!(pulse.slots[0].process, 1);
        assert_eq!(pulse.slots[0].max_height, 8.0);
        assert_eq!(pulse.slots[0].alpha_b, 5);
        assert_eq!(pulse.slots[0].distance, 8.0);
        assert_eq!(pulse.slots[0].full_display_angle, 180);

        for index in 1..4usize {
            assert_eq!(pulse.slots[index].life, 0);
        }
    }

    #[test]
    fn the_shell_spawns_only_on_frame_zero() {
        let mut effect = effect();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 17);

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
    }

    #[test]
    fn the_primitive_aligns_to_the_parent_and_ramps_its_alpha() {
        let mut effect = effect();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 17);
        runtime.parent_angle = 100.0;

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.slots[0].rot_start, 280);
        assert_eq!(runtime.primitives[0].state.slots[0].alpha_b, 5);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.slots[0].process, 2);
        assert_eq!(runtime.primitives[0].state.slots[0].alpha_b, 10);
    }

    #[test]
    fn the_live_primitive_renders_the_half_sphere_quads() {
        let mut effect = effect();
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 17);

        step(&mut effect, &mut runtime);

        // 10 latitude rows x 10 longitude quads -> 200 triangles.
        assert_eq!(render_slots::triangles(&runtime.primitives[0].state).len(), 200);
    }
}
