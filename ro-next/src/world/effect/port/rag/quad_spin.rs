//! `CRagEffect::TexturedQuadSpin(name, phase, kind)` - `RagEffect.cpp:3617-3653`.
//!
//! Spawns four spinning quads (one per `phase`). Called in loops by the
//! Bash3D / Kouenka / Windhawk families.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct TexturedQuadSpin {
    texture: &'static str,
    /// Number of `phase` calls (each spawns four slots).
    count: usize,
    kind: i32,
    lifetime: i32,
}

impl TexturedQuadSpin {
    pub fn new(texture: &'static str, kind: i32, count: usize, lifetime: i32) -> Self {
        Self {
            texture,
            count,
            kind,
            lifetime,
        }
    }

    fn spawn(&self, runtime: &mut EffectRuntime, phase: i32) {
        let kind = self.kind;
        let lifetime = self.lifetime;

        // rotStart is the only random draw per slot.
        let mut rot_starts = [0i16; 4];
        for slot in &mut rot_starts {
            *slot = runtime.rng.clamped_random(360) as i16;
        }

        let spin = runtime.launch(PrimitiveId::SpinQuad, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spin);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.texture = Some(self.texture);
        state.roll = 0.0;

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.height[4] = kind as f32;

            if kind == 2 || kind == 3 || kind == 6 {
                slot.process = 0;
                slot.distance = 2.0;

                slot.rise_angle = if kind == 3 {
                    (phase * 7 + index as i32 * 90) as i16
                } else {
                    (phase * 22 + index as i32 * 90) as i16
                };
            } else {
                slot.process = -24;
                slot.distance = 2.0;
                slot.rise_angle = (phase * 22 + index as i32 * 90) as i16;
            }

            slot.rot_start = rot_starts[index];
            slot.alpha_b = 0;
            slot.height[0] = -12.0;
        }
    }
}

impl PortedEffect for TexturedQuadSpin {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        for phase in 0..self.count as i32 {
            self.spawn(runtime, phase);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn bash3d6_spawns_eight_spinning_quads() {
        let mut effect = TexturedQuadSpin::new("effect\\alpha_center.tga", 6, 8, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 221);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 8);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::SpinQuad);
        assert_eq!(state.duration, 200);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.texture, Some("effect\\alpha_center.tga"));
        assert_eq!(state.slots[0].height[4], 6.0);
        // One motion step has run: process 1 and the kind-6 outward push.
        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].distance, 5.0);
        assert_eq!(state.slots[0].height[0], -12.0);
    }

    #[test]
    fn phases_advance_the_orbit_angle() {
        let mut effect = TexturedQuadSpin::new("effect\\alpha_center.tga", 6, 3, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 221);

        step(&mut effect, &mut runtime);

        // phase 0..2 -> rise angles 0,22,44 for slot 0.
        assert_eq!(runtime.primitives[0].state.slots[0].rise_angle, 0);
        assert_eq!(runtime.primitives[1].state.slots[0].rise_angle, 22);
        assert_eq!(runtime.primitives[2].state.slots[0].rise_angle, 44);
    }

    #[test]
    fn kind_three_uses_the_slower_angle_step() {
        let mut effect = TexturedQuadSpin::new("effect\\alpha_center.tga", 3, 2, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 221);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[1].state.slots[0].rise_angle, 7);
    }
}
