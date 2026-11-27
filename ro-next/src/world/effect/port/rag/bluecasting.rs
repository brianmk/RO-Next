//! `EF_BLUECASTING` (441) - `CRagEffect::BlueSpellRings`,
//! `RagEffect.cpp:18667-18684`.
//!
//! The blue casting rings: a positional `EF_BeginSpell.wav` at height `-100`
//! followed by two `TexturedQuadRingSized` calls (base alphas 45 and 25, both
//! `effect\ring_blue.tga`, kind 5) on frame 0 only.

use super::super::runtime::{EffectRuntime, PortedEffect};
use super::asura::textured_quad_ring_sized;

#[derive(Debug)]
pub struct BlueSpellRings {
    lifetime: i32,
}

impl BlueSpellRings {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for BlueSpellRings {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `CopyVector(this->render.position, this->state.parent->render.position);`
        runtime.copy_parent_position();

        // `if (this->state.stateCnt == 0) { ... }`
        if runtime.frame != 0 {
            return true;
        }

        // `g_soundMgr->PlayWave("effect\\EF_BeginSpell.wav",
        //     render.position.x - player->render.position.x, -100.0f,
        //     render.position.z - player->render.position.z, 250, 40, 1.0f, 0);`
        runtime.play_sound_at_height("effect\\EF_BeginSpell.wav", -100.0);

        // `this->TexturedQuadRingSized(45, "effect\\ring_blue.tga", 5);`
        textured_quad_ring_sized(runtime, 45, "effect\\ring_blue.tga", 5);
        // `this->TexturedQuadRingSized(25, "effect\\ring_blue.tga", 5);`
        textured_quad_ring_sized(runtime, 25, "effect\\ring_blue.tga", 5);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn frame_zero_spawns_the_two_blue_rings_and_the_positional_sound() {
        let mut effect = BlueSpellRings::new(60);
        let mut runtime = EffectRuntime::new(60, [0.0, 0.0], 1);
        runtime.position = [3.0, 4.0, 5.0];
        runtime.listener_position = [1.0, 2.0, 3.0];

        // Call the body directly so the assertions see the spawn frame
        // before the prim-30 motion advances the slots.
        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.primitives.len(), 2);

        for primitive in &runtime.primitives {
            assert_eq!(primitive.state.count, PrimitiveId::Casting3d);
            assert_eq!(primitive.state.texture, Some("effect\\ring_blue.tga"));
            assert_eq!(primitive.state.size, 4.0);
            assert_eq!(primitive.state.render_flag, 1 | 4);
            assert_eq!(primitive.state.duration, 60);
        }

        // Quirk kept by `TexturedQuadRingSized` (`RagEffect.cpp:17598-17603`):
        // a lifetime of at most 60 zeroes every `alphaB` and staggers the
        // slots with `process = index * -5`. Bluecasting's lifetime is exactly
        // 60, so the body's `baseAlpha + 135/90/45/0` seeds are overwritten.
        for primitive in &runtime.primitives {
            for (slot_index, slot) in primitive.state.slots.iter().enumerate() {
                assert_eq!(slot.alpha_b, 0);
                assert_eq!(slot.process, -5 * slot_index as i32);
            }
        }

        assert_eq!(runtime.positional_sounds, vec![("effect\\EF_BeginSpell.wav".to_owned(), [
            2.0, -100.0, 2.0
        ])]);
    }

    #[test]
    fn nothing_spawns_after_frame_zero() {
        let mut effect = BlueSpellRings::new(60);
        let mut runtime = EffectRuntime::new(60, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);
        // Call the body directly so the assertions see the spawn frame
        // before the prim-30 motion advances the slots.
        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.primitives.len(), 2);
    }

    #[test]
    fn the_short_lifetime_is_raised_to_fifty_six_by_the_helper() {
        let mut effect = BlueSpellRings::new(30);
        let mut runtime = EffectRuntime::new(30, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.lifetime, 56);
        assert_eq!(runtime.primitives[0].state.duration, 56);
    }
}
