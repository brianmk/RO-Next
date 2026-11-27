//! `CRagEffect::ShockwaveRing(std::int16_t angle)` - `RagEffect.cpp:2807-2838`,
//! and its only dispatch, `case EF_CHEMICALPROTECTION:` -
//! `RagEffect.cpp:10411-10418`.
//!
//! The chemical-protection shockwave. The dispatch copies the parent anchor,
//! plays `apocalips_attack.wav` (the original passes the bare file name, with
//! no `effect\\` directory) and calls `ShockwaveRing(30)` then
//! `ShockwaveRing(60)`. Each call arms one **prim 54**
//! (`PP_CHEMICALPROTECTION`) primitive and drops the *effect* anchor by 9 units
//! **after** the launch; because each call starts with its own parent copy, the
//! second call resets `y` before it launches, so both primitives sit at the
//! caster and the effect ends 9 units down - not 18.
//!
//! Per primitive:
//!
//! * `renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND`,
//! * texture `effect\\shockwave_c.bmp`,
//! * all four slots live with `process = 0`, `distance = 100`, `alphaB = 0` and
//!   `riseAngle = index * 90 + angle` (30 or 60).
//!
//! Renderer: [`render_chemical`](super::super::render_slots::render_chemical)
//! (`RenderChemical`, `EffectPrimitiveRendering.cpp:6906-6975`, dispatch
//! `:611-612`). Lifetime 300 (`EffectTypes.cpp:309`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// The `EF_CHEMICALPROTECTION` dispatch: wave + `ShockwaveRing(30/60)`.
#[derive(Debug)]
pub struct ChemicalProtection {
    lifetime: i32,
}

impl ChemicalProtection {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

/// `CRagEffect::ShockwaveRing(std::int16_t angle)` - `RagEffect.cpp:2807-2838`.
fn shockwave_ring(runtime: &mut EffectRuntime, angle: i16) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;

    // `this->state.lastPrim = LaunchEffectPrim(54, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::ChemicalProtection, [0.0, 0.0, 0.0]);

    // `this->render.position.y -= 9.0f;` after the launch - the primitive
    // already inherited the pre-drop anchor.
    // `render.position.y -= 9.0f` is nine units *up* in the official Y-down
    // world, i.e. +9 on the port's Y-up anchor.
    runtime.position[1] += 9.0;

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some("effect\\shockwave_c.bmp");

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        slot.life = 1;
        slot.process = 0;
        slot.distance = 100.0;
        slot.rise_angle = (index * 90) as i16 + angle;
        slot.alpha_b = 0;
    }
}

impl PortedEffect for ChemicalProtection {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `case EF_CHEMICALPROTECTION:` - `RagEffect.cpp:10411-10418`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            runtime.copy_parent_position();
            runtime.play_sound("apocalips_attack.wav");
        }

        shockwave_ring(runtime, 30);
        shockwave_ring(runtime, 60);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 1);
        runtime.parent_position = Some([10.0, 20.0, 30.0]);
        runtime
    }

    #[test]
    fn spawns_the_two_shockwave_rings() {
        let mut effect = ChemicalProtection::new(300);
        let mut runtime = runtime();

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["apocalips_attack.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 2);

        for (index, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;

            assert_eq!(state.count, PrimitiveId::ChemicalProtection);
            assert_eq!(state.render_flag, 5);
            assert_eq!(state.duration, 300);
            assert_eq!(state.texture, Some("effect\\shockwave_c.bmp"));
            // The primitive inherits the pre-drop anchor in official
            // (Y-down) space: port y 20 -> -20.
            assert_eq!(state.position, [10.0, -20.0, 30.0]);

            for slot_index in 0..4usize {
                let slot = &state.slots[slot_index];
                assert_eq!(slot.life, 1);
                assert_eq!(slot.process, 1);
                assert_eq!(slot.distance, 100.0);
                assert_eq!(slot.rise_angle, (slot_index * 90) as i16 + if index == 0 { 30 } else { 60 });
            }
        }

        // `PrimChemical` fade-in: the first step adds 18 alpha.
        assert_eq!(runtime.primitives[0].state.slots[0].alpha_b, 18);

        // Both ring calls reset the anchor, so only the last lift survives.
        // `render.position.y -= 9.0f` is nine units *up* in the official
        // Y-down world, i.e. +9 on the port's Y-up anchor.
        assert_eq!(runtime.position, [10.0, 29.0, 30.0]);
    }

    #[test]
    fn later_frames_spawn_nothing() {
        let mut effect = ChemicalProtection::new(300);
        let mut runtime = runtime();

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2);
    }

    #[test]
    fn prim_chemical_fades_in_then_holds() {
        let mut effect = ChemicalProtection::new(300);
        let mut runtime = runtime();

        step(&mut effect, &mut runtime);
        for _ in 0..10 {
            step(&mut effect, &mut runtime);
        }

        let slot = &runtime.primitives[0].state.slots[0];
        // 11 steps: process 11, alpha 18 * 10 (process 1..=10) = 180.
        assert_eq!(slot.process, 11);
        assert_eq!(slot.alpha_b, 180);
        assert_eq!(slot.alpha_t, 360 - 0);
        assert_eq!(runtime.primitives[0].state.slots[1].alpha_t, 90);
    }
}
