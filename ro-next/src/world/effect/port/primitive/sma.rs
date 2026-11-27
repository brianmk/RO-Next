//! `PP_SMA` (120) - `CEffectPrim::PrimSMA`,
//! `EffectPrimitiveMotion.cpp:1442-1468`; dispatch `EffectPrimitiveMotion.cpp:
//! 9508-9509`.
//!
//! The four drifting slots of `CRagEffect::SMA` (`EF_STIN3`, and the `SMA(0)`/
//! `SMA(99)` variants). A slot stays put while its `process` is still `<= 0`
//! (the body seeds it with `index - 3`), turns toward the effect's `deltaPos`
//! on its first processed frame and then advances by `height[1]` along
//! `rotStart` every frame.
//!
//! The renderer is [`render_sma3`](super::super::render_slots::render_sma3),
//! which submits no geometry: it launches `EF_SMA2`/`EF_SMA3`/
//! `EF_SMA_BLACK2` at slot 0's position, and those children are what the
//! effect actually shows.

use super::{EffectPrimitiveState, get_angle};
use crate::world::effect::port::angles::Angles;

/// One frame of `PrimSMA` (`EffectPrimitiveMotion.cpp:1442-1468`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles) {
    for index in 0..4usize {
        if state.slots[index].life != 1 {
            continue;
        }

        state.slots[index].process += 1;

        if state.slots[index].process <= 0 {
            continue;
        }

        if state.slots[index].process == 1 {
            let x = state.delta_pos[0] - state.position[0];
            let z = state.delta_pos[2] - state.position[2];

            state.slots[index].rot_start = get_angle(x, z);
        }

        let slot = &mut state.slots[index];

        slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start as usize);
        slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Sma,
            duration: 200,
            position: [0.0, 0.0, 0.0],
            delta_pos: [0.0, 0.0, 10.0],
            ..Default::default()
        };

        for index in 0..4usize {
            state.slots[index] = EffectPrimitiveSlot {
                life: 1,
                process: index as i32 - 3,
                height: {
                    let mut height = [0.0f32; 21];
                    height[1] = 0.8;
                    height
                },
                ..Default::default()
            };
        }

        state
    }

    #[test]
    fn a_slot_waits_until_its_process_counter_is_positive() {
        let angles = Angles::new();
        let mut state = state();

        // Slots start at -3/-2/-1/0, and the counter is incremented before the
        // `process <= 0` guard. Slot 0 needs four calls to start moving; slot
        // 3 starts on the first one.
        on_process(&mut state, &angles);

        assert_eq!(state.slots[0].process, -2);
        assert_eq!(state.slots[0].vec_b_now, [0.0; 3]);
        assert_eq!(state.slots[3].process, 1);
        assert!((state.slots[3].vec_b_now[2] - 0.8 * angles.raw_sine(91)).abs() < 1e-6);

        on_process(&mut state, &angles);
        on_process(&mut state, &angles);

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].vec_b_now, [0.0; 3], "still waiting");

        on_process(&mut state, &angles);

        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].rot_start, 91);
        assert!((state.slots[0].vec_b_now[2] - 0.8 * angles.raw_sine(91)).abs() < 1e-6);
    }

    #[test]
    fn the_first_processed_frame_turns_toward_delta_pos_then_drifts() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 0;

        on_process(&mut state, &angles);

        // `Get_Angle(0, 10)` is 91: the table's `cos(90)` is a hair above zero,
        // so the first index whose cosine the +z direction exceeds is 91 (the
        // same off-by-one the STIN family keeps).
        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].rot_start, 91);
        assert!((state.slots[0].vec_b_now[0] - 0.8 * angles.raw_cosine(91)).abs() < 1e-6);
        assert!((state.slots[0].vec_b_now[2] - 0.8 * angles.raw_sine(91)).abs() < 1e-6);

        on_process(&mut state, &angles);

        // Once `process > 1` the angle is kept and the slot keeps drifting.
        assert_eq!(state.slots[0].rot_start, 91);
        assert!((state.slots[0].vec_b_now[0] - 1.6 * angles.raw_cosine(91)).abs() < 1e-6);
        assert!((state.slots[0].vec_b_now[2] - 1.6 * angles.raw_sine(91)).abs() < 1e-6);
    }

    #[test]
    fn dead_slots_are_ignored() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[2].life = 0;
        state.slots[2].process = 5;

        on_process(&mut state, &angles);

        assert_eq!(state.slots[2].process, 5);
        assert_eq!(state.slots[2].vec_b_now, [0.0; 3]);
    }
}
