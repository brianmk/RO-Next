//! prim 44 - `CEffectPrim::PrimExpandPulse`,
//! `EffectPrimitiveMotion.cpp:5331-5369`; dispatch `case 44:`
//! `EffectPrimitiveMotion.cpp:9312-9313`.
//!
//! `EffectPrimId.h` has no symbol between `PP_INTIMIDATE` (43) and
//! `PP_TEIHIT2` (45), so the port names the variant after the only motion body
//! the id runs. It is the scatter pulse behind `CRagEffect::TexturedScatter`
//! and `CRagEffect::TexturedScatterLow` (`RagEffect.cpp:3180-3240`), which
//! `EF_TEIHIT1`/`1X`/`1T` and `EF_TEIHIT3` spawn.
//!
//! The body arms four slots with a *negative* `process` delay; each frame the
//! delay advances, and once it crosses zero `distance` grows by `height[0]`
//! (3.0 for `TexturedScatter`, 2.0 for `TexturedScatterLow`) per frame. The
//! alpha ramps up by 15 during `process` 11..=20 and then falls by 5 until it
//! reaches zero; unlike most primitives the slot does **not** retire here -
//! `alphaB == 0` only hides it, and the primitive lives until
//! `stateCnt > duration`. `flag1[1]` is the kind (`16`/`24`) but, with the
//! `TT_MIN_EFFECT_ON_OFF` service option off, all kinds take the same `+15`
//! step.
//!
//! The renderer is
//! [`render_rotated_plate`](super::super::render_slots::render_rotated_plate)
//! (`RenderRotatedPlate`, `EffectPrimitiveRendering.cpp:2747-2784`, dispatch
//! `:542-543`).

use super::EffectPrimitiveState;

/// `g_serviceOptions.Get(TT_MIN_EFFECT_ON_OFF)` defaults to `false`
/// (`ServiceOptions.cpp:499-501`), like every other ported service-option
/// branch.
const MIN_EFFECT_ON_OFF: bool = false;

/// One frame of `PrimExpandPulse` (`EffectPrimitiveMotion.cpp:5331-5369`).
pub fn on_process(state: &mut EffectPrimitiveState) -> bool {
    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process > 0 {
            slot.distance += slot.height[0];
        }

        if slot.process >= 11 && slot.process <= 20 {
            if slot.flag1[1] == 16 {
                slot.alpha_b += 15;
            } else if slot.flag1[1] == 24 {
                if MIN_EFFECT_ON_OFF {
                    slot.alpha_b += 8;
                } else {
                    slot.alpha_b += 15;
                }
            } else {
                slot.alpha_b += 15;
            }
        } else {
            slot.alpha_b -= 5;

            if slot.alpha_b <= 0 {
                slot.alpha_b = 0;
            }
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state(kind: i8) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::ExpandPulse,
            duration: 300,
            ..Default::default()
        };

        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: -10,
            distance: -10.0,
            alpha_b: 0,
            height: {
                let mut height = [0.0; 21];
                height[0] = 3.0;
                height[1] = 0.7;
                height
            },
            flag1: [0; 21],
            ..Default::default()
        };
        state.slots[0].flag1[1] = kind;

        state
    }

    #[test]
    fn primitive_id_is_44() {
        assert_eq!(PrimitiveId::ExpandPulse.id(), 44);
    }

    #[test]
    fn the_delay_advances_before_the_slot_moves() {
        let mut state = state(24);

        // process -10 -> -9: still not past zero, so distance and alpha stay.
        assert!(on_process(&mut state));
        assert_eq!(state.slots[0].process, -9);
        assert_eq!(state.slots[0].distance, -10.0);
        assert_eq!(state.slots[0].alpha_b, 0);

        // Jump to the frame before the first positive process.
        state.slots[0].process = -1;
        on_process(&mut state);

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].distance, -10.0, "process 0 does not move");
        assert_eq!(state.slots[0].alpha_b, 0);

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].distance, -7.0, "distance += height[0]");
    }

    #[test]
    fn alpha_ramps_up_between_frames_eleven_and_twenty_then_falls() {
        let mut state = state(24);
        state.slots[0].process = 9;
        state.slots[0].alpha_b = 0;

        // process 10: the outer `else` still subtracts 5 (clamped at zero).
        on_process(&mut state);
        assert_eq!(state.slots[0].process, 10);
        assert_eq!(state.slots[0].alpha_b, 0);

        // process 11..=20: +15 per frame.
        for expected in 1..=10i16 {
            on_process(&mut state);
            assert_eq!(state.slots[0].alpha_b, expected * 15);
        }
        assert_eq!(state.slots[0].process, 20);
        assert_eq!(state.slots[0].alpha_b, 150);

        // process 21: -5.
        on_process(&mut state);
        assert_eq!(state.slots[0].process, 21);
        assert_eq!(state.slots[0].alpha_b, 145);

        // It never retires through alpha; 29 more frames reach zero.
        for _ in 0..29 {
            on_process(&mut state);
        }
        assert_eq!(state.slots[0].process, 50);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 1, "the slot is hidden but still alive");
    }

    #[test]
    fn kind_16_also_gets_the_full_alpha_step() {
        let mut state = state(16);
        state.slots[0].process = 10;
        on_process(&mut state);
        assert_eq!(state.slots[0].alpha_b, 15);
    }

    #[test]
    fn dead_slots_are_untouched() {
        let mut state = state(24);
        state.slots[0].life = 0;

        on_process(&mut state);

        assert_eq!(state.slots[0].process, -10);
        assert_eq!(state.slots[0].distance, -10.0);
    }
}
