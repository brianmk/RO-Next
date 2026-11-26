//! prim 103 (`EffectPrimId.h` has no symbol for it; the switches use
//! `case 103`) - `CEffectPrim::PrimSpinFade`,
//! `EffectPrimitiveMotion.cpp:5299-5331`.
//!
//! The motion of `CRagEffect::TexturedMark`'s stacked rings. One live slot:
//! every active frame the slot turns by `rand() % 3 + 3` degrees and its alpha
//! ramps up for 40 frames, then fades out (after 60 frames, or 120 for the
//! `flag1[5] == 1` "tall" variant; `flag1[5] == 10` never fades). The renderer
//! reads the accumulated `rotStart`/`alphaB` and the slot's rings.
//!
//! The motion consumes one `std::rand() % 3` per live slot per frame, in slot
//! order, exactly like the original.

use super::super::rng::EffectRng;
use super::EffectPrimitiveState;

/// One frame of `PrimSpinFade`.
pub fn on_process(state: &mut EffectPrimitiveState, rng: &mut EffectRng) {
    for index in 0..4usize {
        // `if (slots[index].life != 1 || ++slots[index].process <= 0) continue;`
        // The increment happens before the check, so an inactive slot's
        // `process` is not touched.
        if state.slots[index].life != 1 {
            continue;
        }

        state.slots[index].process += 1;

        if state.slots[index].process <= 0 {
            continue;
        }

        // `const int turn = std::rand() % 3;`
        // `(turn >= 0 ? turn : 0)` is always `turn` for a non-negative
        // `rand()`, so the flat `% 3` draw is kept.
        let turn = rng.clamped_random(3);

        let slot = &mut state.slots[index];
        // `static_cast<std::int16_t>(rotStart + (turn >= 0 ? turn : 0) + 3)`
        slot.rot_start = (slot.rot_start + (turn as i16) + 3) as i16;

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        if slot.flag1[5] == 1 {
            if slot.process > 120 {
                slot.alpha_b -= 1;
            }
        } else if slot.flag1[5] != 10 && slot.process > 60 {
            slot.alpha_b -= 1;
        }

        if slot.process <= 40 {
            slot.alpha_b += 1;
        }

        if slot.alpha_b < 0 {
            slot.alpha_b = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state(kind: i8) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::StackedRings,
            duration: 200,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            rot_start: 100,
            flag1: {
                let mut flag1 = [0i8; 21];
                flag1[5] = kind;
                flag1
            },
            ..Default::default()
        };
        // Slots 1..3 stay inactive.
        state
    }

    #[test]
    fn primitive_id_is_103() {
        assert_eq!(PrimitiveId::StackedRings.id(), 103);
    }

    #[test]
    fn each_live_slot_advances_once_per_frame() {
        let mut state = state(0);
        let mut rng = EffectRng::new(7);

        on_process(&mut state, &mut rng);
        assert_eq!(state.slots[0].process, 1);

        on_process(&mut state, &mut rng);
        assert_eq!(state.slots[0].process, 2);
    }

    #[test]
    fn rotation_is_rand_three_plus_three() {
        // `rotStart + (rand() % 3) + 3`, wrapped once past 360.
        for seed in 1..50u64 {
            let mut expected = EffectRng::new(seed);
            let turn = expected.clamped_random(3) as i16;

            let mut state = state(0);
            let mut rng = EffectRng::new(seed);
            on_process(&mut state, &mut rng);

            assert_eq!(state.slots[0].rot_start, 100 + turn + 3, "seed {seed}");
        }
    }

    #[test]
    fn rotation_wraps_after_one_full_turn() {
        let mut state = state(0);
        state.slots[0].rot_start = 359;
        let mut expected = EffectRng::new(11);
        let turn = expected.clamped_random(3) as i16;

        let mut rng = EffectRng::new(11);
        on_process(&mut state, &mut rng);

        // 359 + 3 + turn is always >= 360, so one subtraction applies.
        assert_eq!(state.slots[0].rot_start, 359 + 3 + turn - 360);
    }

    #[test]
    fn alpha_ramps_for_the_first_forty_frames() {
        let mut state = state(0);
        state.slots[0].alpha_b = 0;
        let mut rng = EffectRng::new(3);

        for frame in 1..=40 {
            on_process(&mut state, &mut rng);
            assert_eq!(state.slots[0].alpha_b, frame as i16);
        }

        // Frame 41 is the first without the +1 and still below the fade gate.
        on_process(&mut state, &mut rng);
        assert_eq!(state.slots[0].alpha_b, 40);
    }

    #[test]
    fn plain_kind_fades_only_after_sixty_frames() {
        let mut state = state(0);
        state.slots[0].alpha_b = 100;
        state.slots[0].process = 59;
        let mut rng = EffectRng::new(3);

        on_process(&mut state, &mut rng);
        assert_eq!(state.slots[0].process, 60);
        assert_eq!(state.slots[0].alpha_b, 100);

        on_process(&mut state, &mut rng);
        assert_eq!(state.slots[0].process, 61);
        assert_eq!(state.slots[0].alpha_b, 99);
    }

    #[test]
    fn flag1_five_one_fades_after_one_hundred_twenty_frames() {
        let mut state = state(1);
        state.slots[0].alpha_b = 100;
        state.slots[0].process = 119;
        let mut rng = EffectRng::new(3);

        on_process(&mut state, &mut rng);
        assert_eq!(state.slots[0].process, 120);
        assert_eq!(state.slots[0].alpha_b, 100);

        on_process(&mut state, &mut rng);
        assert_eq!(state.slots[0].process, 121);
        assert_eq!(state.slots[0].alpha_b, 99);
    }

    #[test]
    fn flag1_five_ten_never_fades() {
        let mut state = state(10);
        state.slots[0].alpha_b = 100;
        state.slots[0].process = 500;
        let mut rng = EffectRng::new(3);

        on_process(&mut state, &mut rng);

        assert_eq!(state.slots[0].process, 501);
        assert_eq!(state.slots[0].alpha_b, 100);
    }

    #[test]
    fn alpha_never_goes_negative() {
        let mut state = state(0);
        state.slots[0].alpha_b = 0;
        state.slots[0].process = 100;
        let mut rng = EffectRng::new(3);

        on_process(&mut state, &mut rng);

        assert_eq!(state.slots[0].alpha_b, 0);
    }

    #[test]
    fn only_live_slots_consume_a_random_draw() {
        // Two live slots -> two draws, in slot order.
        let mut state = state(0);
        state.slots[1] = state.slots[0];
        state.slots[1].rot_start = 0;

        let mut expected = EffectRng::new(19);
        let first = expected.clamped_random(3) as i16;
        let second = expected.clamped_random(3) as i16;

        let mut rng = EffectRng::new(19);
        on_process(&mut state, &mut rng);

        assert_eq!(state.slots[0].rot_start, 100 + first + 3);
        assert_eq!(state.slots[1].rot_start, second + 3);
        // The third slot is inactive: no draw, so the next one is the second
        // live slot's.
        assert_eq!(state.slots[2].process, 0);
    }
}
