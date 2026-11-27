//! `case EF_BEGINSPELL6:` - `RagEffect.cpp:23584-23591`.
//!
//! The default cast-start effect of every skill whose
//! `CSession::GetSkillActionInfo` leaves `effectId` at `EF_BEGINSPELL6` (58):
//!
//! ```cpp
//! case EF_BEGINSPELL6:
//!   this->render.position = this->state.parent->render.position;
//!   if (this->state.stateCnt == 0) {
//!     PlayEffectWaveInline(*this, "effect\\EF_BeginSpell.wav");
//!     this->TexturedQuadRingSized(45, "effect\\ring_white.tga", 0);
//!     this->TexturedQuadRingSized(25, "effect\\ring_white.tga", 0);
//!   }
//!   break;
//! ```
//!
//! Frame 0 anchors the effect on the caster, plays the inline
//! `effect\EF_BeginSpell.wav` and arms two prim-30 ring sets
//! (`CRagEffect::TexturedQuadRingSized`, `RagEffect.cpp:17437-17608`) at base
//! alphas 45 and 25, both `effect\ring_white.tga`, kind 0. The official
//! lifetime is 40 (`EffectTypes.cpp:67`); the helper's `duration < 56` bump
//! raises it to 56 on frame 0. A skill with a cast time additionally extends
//! the duration to the cast length through `AM_MAKE_BEGIN_SPELL` /
//! `AM_SET_DURATION` (`GameObject.cpp:1028-1048`), which the client does not
//! model for ported bodies (the port plays the default 56-frame window).
//!
//! Wind Walk uses it as its "on use" lane: `GetSkillActionInfo` maps
//! `SN_WINDWALK` (383) to `EF_BEGINSPELL6` (`SkillActionInfo.cpp:1750`, the
//! default `ShowsCastEffect` is true, `SkillInfo.cpp:890-927`), and
//! `CGameMode::NotifySkillCasting` launches it through
//! `LaunchSkillActorEffect` (`GameModePackets.cpp:12947-12953`,
//! `:17551-17559`).

use super::super::runtime::{EffectRuntime, PortedEffect};
use super::asura::textured_quad_ring_sized;

#[derive(Debug)]
pub struct BeginSpellRings {
    lifetime: i32,
}

impl BeginSpellRings {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for BeginSpellRings {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `this->render.position = this->state.parent->render.position;`
        runtime.copy_parent_position();

        // `if (this->state.stateCnt == 0) { ... }`
        if runtime.frame != 0 {
            return true;
        }

        // `PlayEffectWaveInline(*this, "effect\\EF_BeginSpell.wav");`
        runtime.play_sound("effect\\EF_BeginSpell.wav");

        // `this->TexturedQuadRingSized(45, "effect\\ring_white.tga", 0);`
        textured_quad_ring_sized(runtime, 45, "effect\\ring_white.tga", 0);
        // `this->TexturedQuadRingSized(25, "effect\\ring_white.tga", 0);`
        textured_quad_ring_sized(runtime, 25, "effect\\ring_white.tga", 0);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;
    use crate::world::effect::port::runtime::step;

    /// Every official layer: the inline begin-spell wave and the two white
    /// prim-30 ring sets (base alphas 45 and 25).
    #[test]
    fn frame_zero_spawns_the_wave_and_the_two_white_ring_sets() {
        let mut effect = BeginSpellRings::new(40);
        let mut runtime = EffectRuntime::new(40, [0.0, 0.0], 1);

        // Call the body directly so the assertions see the spawn frame before
        // the prim-30 motion advances the slots.
        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.sounds, vec!["effect\\EF_BeginSpell.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 2, "one ring set per base alpha");

        for primitive in &runtime.primitives {
            assert_eq!(primitive.state.count, PrimitiveId::Casting3d);
            assert_eq!(primitive.state.texture, Some("effect\\ring_white.tga"));
            // Kind 0 selects the default colour table entry (`5.0`).
            assert_eq!(primitive.state.size, 5.0);
            assert_eq!(primitive.state.render_flag, 1 | 4);
            // `duration < 56` raises the effect and every ring primitive.
            assert_eq!(primitive.state.duration, 56);
            assert_eq!(runtime.lifetime, 56);

            // The `lifetime <= 60` quirk zeroes `alphaB` and staggers the
            // slots with `process = index * -5`, so both base alphas (45 and
            // 25) are overwritten - the two ring sets are visually identical.
            for (slot_index, slot) in primitive.state.slots.iter().enumerate() {
                assert_eq!(slot.alpha_b, 0);
                assert_eq!(slot.process, -5 * slot_index as i32);
                assert_eq!(slot.alpha_t, 0, "kind 0 is carried in alphaT");
            }
        }
    }

    /// The rings are a frame-0 layer only; later frames neither re-arm the
    /// primitives nor replay the wave.
    #[test]
    fn nothing_spawns_after_frame_zero() {
        let mut effect = BeginSpellRings::new(40);
        let mut runtime = EffectRuntime::new(40, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);
        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.primitives.len(), 2);
        assert_eq!(runtime.sounds, vec!["effect\\EF_BeginSpell.wav".to_owned()]);
    }

    /// The body anchors on the caster's position on frame 0.
    #[test]
    fn frame_zero_copies_the_caster_position() {
        let mut effect = BeginSpellRings::new(40);
        let mut runtime = EffectRuntime::new(40, [0.0, 0.0], 1);
        runtime.position = [10.0, 20.0, 30.0];
        runtime.parent_position = Some([1.0, 2.0, 3.0]);

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.position, [1.0, 2.0, 3.0]);
    }
}
