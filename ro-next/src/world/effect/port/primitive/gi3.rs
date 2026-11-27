//! prim 34 `PP_GI_3` - `CEffectPrim::PrimGI3`,
//! `EffectPrimitiveMotion.cpp:2440-2473`; dispatch `case PP_GI_3:`
//! `EffectPrimitiveMotion.cpp:8921-8922`.
//!
//! One of the two primitives of Grand Cross: the four white/black rings that
//! contract onto the caster and then flash out. `PrimGI3` advances each live
//! slot once per frame, shrinks `distance` by `0.01` and fills every height
//! sample with `maxHeight * g_sinTable[process]`:
//!
//! ```cpp
//! slot.process += 1;
//! slot.distance -= 0.01f;
//! if (slot.process >= 10) {
//!   slot.alphaB -= 1;
//!   if (slot.alphaB <= 0) {
//!     slot.alphaB = 0;
//!     if (slot.process > 30) slot.life = 0;
//!     slot.distance = 20.0f;
//!     slot.riseAngle = 81 + 9;
//!   }
//! } else {
//!   slot.alphaB += 10;
//! }
//! for (sample) slot.height[sample] = slot.maxHeight * g_sinTable[slot.process];
//! ```
//!
//! Quirks kept: the death test is `process > 30` **and** `alphaB <= 0`, so with
//! the bodies' `alphaB = 0` start the slot survives until the ramp reaches zero
//! at `process` 99, `distance` keeps shrinking all the way there, and the
//! `distance`/`riseAngle` reset happens on the same frame the slot dies.
//! The renderer is
//! [`render_3d_casting2`](super::super::render_slots::render_3d_casting2)
//! (`Render3DCasting2`, `EffectPrimitiveRendering.cpp:6519-6592`, dispatch
//! `:521-522`).

use super::super::angles::Angles;
use super::EffectPrimitiveState;

/// One frame of `PrimGI3` (`EffectPrimitiveMotion.cpp:2440-2473`).
pub fn on_process(state: &mut EffectPrimitiveState) -> bool {
    let angles = Angles::shared();

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;
        slot.distance -= 0.01;

        if slot.process >= 10 {
            slot.alpha_b -= 1;

            if slot.alpha_b <= 0 {
                slot.alpha_b = 0;

                if slot.process > 30 {
                    slot.life = 0;
                }

                slot.distance = 20.0;
                slot.rise_angle = 81 + 9;
            }
        } else {
            slot.alpha_b += 10;
        }

        for sample in 0..21usize {
            slot.height[sample] = slot.max_height * angles.raw_sine(slot.process as usize);
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Gi3,
            duration: 999,
            ..Default::default()
        };

        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: 0,
            full_display_angle: 90,
            max_height: 120.0,
            alpha_b: 0,
            distance: 19.9,
            rise_angle: 90,
            ..Default::default()
        };
        state.slots[0].flag1[4] = 0;

        state
    }

    #[test]
    fn primitive_id_is_34() {
        assert_eq!(PrimitiveId::Gi3.id(), 34);
    }

    #[test]
    fn the_ring_contracts_and_ramps_alpha_for_nine_frames() {
        let angles = Angles::shared();
        let mut state = state();

        for frame in 1..=9 {
            assert!(on_process(&mut state));

            let slot = &state.slots[0];
            assert_eq!(slot.process, frame);
            assert_eq!(slot.alpha_b, (frame * 10) as i16);
            // `19.9 - 0.01 * frame`, evaluated with the same repeated step.
            let mut distance = 19.9f32;
            for _ in 0..frame {
                distance -= 0.01;
            }
            assert_eq!(slot.distance, distance);
            assert_eq!(slot.height[0], 120.0 * angles.raw_sine(frame as usize));
            assert_eq!(slot.height[20], slot.height[0], "all 21 samples are filled");
        }

        assert_eq!(state.slots[0].life, 1);
    }

    #[test]
    fn alpha_falls_one_per_frame_after_process_nine() {
        let mut state = state();
        state.slots[0].process = 9;
        state.slots[0].alpha_b = 90;
        state.slots[0].distance = 19.81;

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 10);
        assert_eq!(state.slots[0].alpha_b, 89);
        assert!((state.slots[0].distance - 19.8).abs() < 1e-5);
    }

    #[test]
    fn the_slot_dies_only_when_the_alpha_ramp_reaches_zero_after_frame_thirty() {
        let mut state = state();
        state.slots[0].process = 98;
        state.slots[0].alpha_b = 1;
        state.slots[0].distance = 4.2;

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 99);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 0, "process 99 > 30");
        assert_eq!(state.slots[0].distance, 20.0, "the reset is in the death branch");
        assert_eq!(state.slots[0].rise_angle, 90);
    }

    #[test]
    fn a_slot_that_hits_zero_alpha_on_or_before_frame_thirty_survives() {
        let mut state = state();
        // Alpha reaches zero at process 20, but the death test fails.
        state.slots[0].process = 19;
        state.slots[0].alpha_b = 1;

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 20);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 1);
        assert_eq!(state.slots[0].distance, 20.0);
    }

    #[test]
    fn dead_slots_are_untouched() {
        let mut state = state();
        state.slots[0].life = 0;

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].distance, 19.9);
        assert_eq!(state.slots[0].height[0], 0.0);
    }
}
