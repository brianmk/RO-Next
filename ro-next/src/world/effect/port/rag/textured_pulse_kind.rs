//! `CRagEffect::TexturedPulseKind(name, kind)` - `RagEffect.cpp:11135-11207`.
//!
//! `EF_BOTTOM_MAG` (kind 0) and `EF_BOTTOM_SANC` (kind 1): a single
//! `PP_PULSEKIND` (64) slot with kind-specific `maxHeight`/`alphaB`/`alphaT`.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TexturedPulseKind {
    texture: &'static str,
    kind: i32,
    lifetime: i32,
}

impl TexturedPulseKind {
    pub fn new(texture: &'static str, kind: i32, lifetime: i32) -> Self {
        Self {
            texture,
            kind,
            lifetime,
        }
    }
}

impl PortedEffect for TexturedPulseKind {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let kind = self.kind;
        let lifetime = runtime.lifetime;
        let display = runtime.rng.clamped_random(360) as i16;
        let kind4_height = runtime.rng.clamped_random(50) as f32;
        let mut pulsing = true;

        let pulse = runtime.launch(PrimitiveId::PulseKind, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(pulse);

        state.render_flag = 5; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some(self.texture);
        state.size = 1.0;

        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].full_display_angle = display;

        if kind == 0 {
            // The low-effect variant uses 10; full detail uses 50.
            state.slots[0].max_height = 50.0;
            state.slots[0].rot_start = 0;
            state.slots[0].alpha_b = 30;
            state.slots[0].alpha_t = 864;
            state.slots[0].height[0] = 5.0;
            state.slots[0].height[1] = 5.0;
        } else {
            if kind == 1 {
                state.slots[0].max_height = 16.0;
                state.slots[0].rot_start = 0;
                state.slots[0].alpha_b = 120;
                state.slots[0].alpha_t = 1880;
            } else if kind == 2 {
                state.slots[0].max_height = 32.0;
                state.slots[0].rot_start = 0;
                state.slots[0].alpha_b = 30;
                state.slots[0].alpha_t = 29_999;
            } else if kind == 4 {
                state.slots[0].max_height = 16.0;
                state.slots[0].rot_start = 0;
                state.slots[0].alpha_b = 120;
                state.slots[0].alpha_t = 1880;
                state.slots[0].height[0] = 2.5;
                state.slots[0].height[1] = 2.5;
                state.slots[0].height[2] = 0.0;
                state.slots[0].height[3] = 4.0;
                state.slots[0].height[4] = kind4_height;
                pulsing = false;
            } else {
                if kind == 3 {
                    state.slots[0].max_height = 16.0;
                }

                state.slots[0].rot_start = 0;
            }

            if pulsing {
                state.slots[0].height[0] = 2.5;
                state.slots[0].height[1] = 2.5;
            }
        }

        if pulsing {
            state.slots[0].height[2] = 0.0;
            state.slots[0].height[3] = kind as f32;
        }

        state.slots[0].height[5] = 0.0;
        state.slots[0].height[6] = 0.0;
        state.slots[1].life = 0;
        state.slots[2].life = 0;
        state.slots[3].life = 0;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn bottom_mag_uses_kind_zero() {
        let mut effect = TexturedPulseKind::new("effect\\ring_red.tga", 0, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 471);

        step(&mut effect, &mut runtime);

        let pulse = &runtime.primitives[0].state;
        assert_eq!(pulse.count, PrimitiveId::PulseKind);
        assert_eq!(pulse.render_flag, 5);
        assert_eq!(pulse.size, 1.0);
        assert_eq!(pulse.slots[0].max_height, 50.0);
        assert_eq!(pulse.slots[0].alpha_b, 30);
        assert_eq!(pulse.slots[0].alpha_t, 864);
        assert_eq!(pulse.slots[0].height[0], 5.0);
        assert_eq!(pulse.slots[0].height[3], 0.0);
        assert_eq!(pulse.slots[1].life, 0);
    }

    #[test]
    fn bottom_sanc_uses_kind_one() {
        let mut effect = TexturedPulseKind::new("effect\\alpha_down.tga", 1, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 471);

        step(&mut effect, &mut runtime);

        let pulse = &runtime.primitives[0].state;
        assert_eq!(pulse.slots[0].max_height, 16.0);
        assert_eq!(pulse.slots[0].alpha_b, 120);
        assert_eq!(pulse.slots[0].alpha_t, 1880);
        assert_eq!(pulse.slots[0].height[3], 1.0);
    }
}
