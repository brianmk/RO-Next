//! Prim 137 (`PrimHeightRamp`) - `EffectPrimitiveMotion.cpp:4970-5008`.
//!
//! The rising ring columns of `TexturedSpreadFlat` (Redline/Blueline). Slots
//! ramp their height through `g_sinTable` between process 100 and 115, fade
//! after 125 and rotate one degree per frame.

use super::super::angles::Angles;
use super::EffectPrimitiveState;

/// One frame of `PrimHeightRamp`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles) {
    for index in 0..4 {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];

        slot.process += 1;
        slot.rot_start += 1;

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        if slot.process >= 125 {
            slot.alpha_b -= 15;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }

            slot.max_height -= 5.0;

            if slot.max_height < 0.0 {
                slot.max_height = 0.0;
            }
        }

        for step in 0..21 {
            if slot.process < 100 {
                slot.height[step] = 0.0;
            } else if slot.process <= 115 {
                slot.height[step] = angles.raw_sine((6 * slot.process - 600) as usize) * slot.max_height;
            } else {
                slot.height[step] = slot.max_height;
            }

            if slot.height[step] < 0.0 {
                slot.height[step] = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn slot_state(process: i32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::HeightRamp,
            duration: 200,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process,
            max_height: 120.0,
            alpha_b: 150,
            ..Default::default()
        };
        state
    }

    #[test]
    fn heights_stay_zero_before_process_hundred() {
        let angles = Angles::new();
        let mut state = slot_state(50);

        on_process(&mut state, &angles);

        assert_eq!(state.slots[0].height[0], 0.0);
        assert_eq!(state.slots[0].process, 51);
    }

    #[test]
    fn heights_ramp_through_the_sine_table() {
        let angles = Angles::new();
        let mut state = slot_state(100);

        on_process(&mut state, &angles);

        // process 101 -> sin(6) * 120
        assert!((state.slots[0].height[0] - angles.raw_sine(6) * 120.0).abs() < 1e-6);
    }

    #[test]
    fn heights_clamp_to_max_after_process_115() {
        let angles = Angles::new();
        let mut state = slot_state(116);

        on_process(&mut state, &angles);

        assert_eq!(state.slots[0].height[0], 120.0);
    }

    #[test]
    fn fade_starts_at_one_hundred_twenty_five() {
        let angles = Angles::new();
        let mut state = slot_state(125);

        on_process(&mut state, &angles);

        assert_eq!(state.slots[0].alpha_b, 135);
        assert_eq!(state.slots[0].max_height, 115.0);
    }
}
