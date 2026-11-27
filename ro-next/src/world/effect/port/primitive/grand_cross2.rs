//! prim 37 `PP_GRANDCROSS2` - the motion branch of `CEffectPrim` for the
//! Grand Cross cross quad, `EffectPrimitiveMotion.cpp:9184-9211`; dispatch
//! `case PP_GRANDCROSS2:`.
//!
//! Unlike the other Grand Cross primitive (34, [`super::gi3`]), the official
//! `switch` has no member function for this id: the body is the bare `case`
//! block. It advances each live slot, ramps `alphaB` by `+9` for the first nine
//! frames and then by `-1`, retires the slot when the alpha has been at zero
//! past frame 30, and feeds the three heights the `RenderGrandCross2` quad
//! consumes:
//!
//! ```cpp
//! ++slot.process;
//! if (slot.process < 10) { slot.alphaB += 9; }
//! else { --slot.alphaB; if (slot.alphaB < 1) { ... if (slot.process > 30) slot.life = 0; } }
//! slot.height[1] += 0.005f;
//! slot.height[0] += 0.005f;
//! slot.height[2] = g_sinTable[slot.process] * slot.maxHeight;
//! ```
//!
//! The renderer is
//! [`render_grand_cross2`](super::super::render_slots::render_grand_cross2)
//! (`RenderGrandCross2`, `EffectPrimitiveRendering.cpp:7365-7421`, dispatch
//! `:524-525`).

use super::super::angles::Angles;
use super::EffectPrimitiveState;

/// One frame of the official `case PP_GRANDCROSS2`
/// (`EffectPrimitiveMotion.cpp:9184-9211`).
pub fn on_process(state: &mut EffectPrimitiveState) -> bool {
    let angles = Angles::shared();

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process < 10 {
            slot.alpha_b += 9;
        } else {
            slot.alpha_b -= 1;

            if slot.alpha_b < 1 {
                slot.alpha_b = 0;

                if slot.process > 30 {
                    slot.life = 0;
                }
            }
        }

        // The order is the original's: `height[1]`, then `height[0]`, then the
        // sine sample in `height[2]`.
        slot.height[1] += 0.005;
        slot.height[0] += 0.005;
        slot.height[2] = angles.raw_sine(slot.process as usize) * slot.max_height;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::GrandCross2,
            duration: 999,
            ..Default::default()
        };

        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: 0,
            max_height: 60.0,
            rot_start: 0,
            alpha_b: 0,
            height: {
                let mut height = [0.0; 21];
                height[0] = 24.0;
                height[1] = 3.2;
                height[2] = 0.0;
                height
            },
            ..Default::default()
        };

        state
    }

    #[test]
    fn primitive_id_is_37() {
        assert_eq!(PrimitiveId::GrandCross2.id(), 37);
    }

    #[test]
    fn alpha_ramps_up_for_nine_frames() {
        let mut state = state();

        for frame in 1..=9 {
            assert!(on_process(&mut state));
            assert_eq!(state.slots[0].process, frame);
            assert_eq!(state.slots[0].alpha_b, (frame * 9) as i16);
        }

        assert_eq!(state.slots[0].life, 1);
    }

    #[test]
    fn alpha_falls_one_per_frame_from_process_ten() {
        let mut state = state();
        state.slots[0].process = 9;
        state.slots[0].alpha_b = 81;

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 10);
        assert_eq!(state.slots[0].alpha_b, 80);
    }

    #[test]
    fn the_heights_accumulate_in_the_official_order() {
        let mut state = state();

        on_process(&mut state);
        on_process(&mut state);

        // Repeated `+= 0.005` twice, exactly like the body.
        let mut expected0 = 24.0f32;
        expected0 += 0.005;
        expected0 += 0.005;
        let mut expected1 = 3.2f32;
        expected1 += 0.005;
        expected1 += 0.005;

        assert_eq!(state.slots[0].height[0], expected0);
        assert_eq!(state.slots[0].height[1], expected1);
    }

    #[test]
    fn the_sine_sample_uses_the_process_count() {
        let angles = Angles::shared();
        let mut state = state();
        state.slots[0].process = 44;

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 45);
        assert_eq!(state.slots[0].height[2], angles.raw_sine(45) * 60.0);
    }

    #[test]
    fn the_slot_retires_at_zero_alpha_after_frame_thirty() {
        let mut state = state();
        state.slots[0].process = 90;
        state.slots[0].alpha_b = 1;

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 91);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 0);
    }

    #[test]
    fn a_dead_slot_is_untouched() {
        let mut state = state();
        state.slots[0].life = 0;

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].height[0], 24.0);
        assert_eq!(state.slots[0].height[1], 3.2);
    }
}
