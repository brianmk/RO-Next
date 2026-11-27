//! `PP_SMA2` (121) - the bare `case PP_SMA2:` block of
//! `CEffectPrim::OnProcess`, `EffectPrimitiveMotion.cpp:9511-9574`.
//!
//! The four stacked wave rings of `CRagEffect::TexturedBlueRing` (`EF_SMA2`)
//! and `CRagEffect::BlueRingStack` (`EF_SMA_BLACK2`). Each slot turns by
//! `3 + index` degrees per frame (one `> 359` wrap, no negative side), fades in
//! to 180 over the first 40 processed frames, fades out again once the slot
//! passes `duration - 40`, and fills the 21 `height[]` samples with a sine
//! wave that grows up to `sin(value * 9) * maxHeight` and is then frozen by
//! `flag1[value]`.
//!
//! `deltaPos2.x` is the launching slot's `process` counter (`RenderSMA3`'s
//! `AM_SETPOS` extra vector); above 50 it caps the alpha with
//! `200 - (x - 50) * 5`, which is what fades the whole black ring stack out as
//! the launcher advances.
//!
//! The renderer is [`render_sma2`](super::super::render_slots::render_sma2).

use super::EffectPrimitiveState;
use crate::world::effect::port::angles::Angles;

/// One frame of `case PP_SMA2` (`EffectPrimitiveMotion.cpp:9511-9574`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles) {
    let duration = state.duration;
    let delta_pos2_x = state.delta_pos2[0];

    for index in 0..4usize {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];

        slot.process += 1;
        slot.rot_start += 3 + index as i16;

        if slot.rot_start > 359 {
            slot.rot_start -= 360;
        }

        if slot.process < duration - 40 {
            if slot.process < 40 {
                slot.alpha_b += 5;

                if slot.alpha_b > 180 {
                    slot.alpha_b = 180;
                }
            }
        } else {
            slot.alpha_b -= 5;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }

        if delta_pos2_x > 50.0 {
            let limit = 200.0 - (delta_pos2_x - 50.0) * 5.0;

            if limit < slot.alpha_b as f32 {
                slot.alpha_b = limit as i16;
            }

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }

        for value in 0..21usize {
            if slot.flag1[value] != 0 {
                continue;
            }

            if slot.process < 91 {
                slot.height[value] = angles.raw_sine(value * 9) * slot.max_height * angles.raw_sine(slot.process as usize);
            }

            let limit = angles.raw_sine(value * 9) * slot.max_height;

            if limit < slot.height[value] {
                slot.height[value] = limit;
                slot.flag1[value] = 1;
            }

            if slot.height[value] < 0.0 {
                slot.height[value] = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Sma2,
            duration: 100,
            ..Default::default()
        };

        for index in 0..3usize {
            state.slots[index] = EffectPrimitiveSlot {
                life: 1,
                max_height: 13.0 - 2.0 * index as f32,
                rot_start: (index * 90) as i16,
                ..Default::default()
            };
        }

        state
    }

    #[test]
    fn each_slot_turns_by_three_plus_its_index() {
        let angles = Angles::new();
        let mut state = state();

        on_process(&mut state, &angles);

        assert_eq!(state.slots[0].rot_start, 3);
        assert_eq!(state.slots[1].rot_start, 94);
        assert_eq!(state.slots[2].rot_start, 185);
        assert_eq!(state.slots[3].life, 0);
    }

    #[test]
    fn alpha_fades_in_to_one_hundred_eighty_during_the_first_forty_frames() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 38;
        state.slots[0].alpha_b = 178;

        on_process(&mut state, &angles);
        assert_eq!(state.slots[0].alpha_b, 180, "process 39 still ramps and clamps");

        // `process` 40 takes neither the `process < 40` ramp nor the fade.
        on_process(&mut state, &angles);
        assert_eq!(state.slots[0].alpha_b, 180);
    }

    #[test]
    fn alpha_fades_out_past_duration_minus_forty() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 59;
        state.slots[0].alpha_b = 180;

        // process becomes 60, which is not `< duration - 40` (60) -> fade.
        on_process(&mut state, &angles);
        assert_eq!(state.slots[0].alpha_b, 175);
    }

    #[test]
    fn the_launcher_extra_caps_the_alpha_above_fifty() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 39;
        state.slots[0].alpha_b = 180;
        // `200 - (60 - 50) * 5 = 150`.
        state.delta_pos2[0] = 60.0;

        on_process(&mut state, &angles);

        assert_eq!(state.slots[0].alpha_b, 150);
    }

    #[test]
    fn heights_grow_until_process_ninety_then_stay() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 44;

        // `process` 45: sin(90) * 13 * sin(45) is about 9.19.
        on_process(&mut state, &angles);
        let half = state.slots[0].height[10];
        assert!(half > 9.0 && half < 9.3, "{half}");
        assert_eq!(state.slots[0].flag1[10], 0);

        // `process` 90 is the last frame the heights are recomputed at, and it
        // reaches the sine limit exactly.
        state.slots[0].process = 89;
        on_process(&mut state, &angles);
        assert!((state.slots[0].height[10] - 13.0).abs() < 1e-3);
        // `process` 0: `sin(0) * maxHeight * sin(90)` = 0.
        assert!((state.slots[0].height[0]).abs() < 1e-6);

        let frozen = state.slots[0].height[10];
        state.slots[0].process = 300;
        on_process(&mut state, &angles);

        assert_eq!(state.slots[0].height[10], frozen, "past process 90 the samples stay");
    }
}
