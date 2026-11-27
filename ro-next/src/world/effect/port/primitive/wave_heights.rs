//! Prim 149 (no `EffectPrimId.h` symbol) - `CEffectPrim::PrimWaveHeights`,
//! `EffectPrimitiveMotion.cpp:5534-5583`; dispatch
//! `EffectPrimitiveMotion.cpp:8308-8309`.
//!
//! The four ground arcs of `CRagEffect::BeginSpellSwirl`
//! (`EF_BEGINSPELL_N*`): each slot turns three degrees per frame, ramps its
//! alpha to 100 during the first thirty processed frames, fades it again once
//! it passes `duration - 46`, and fills the 21 `height[]` samples with a
//! travelling sine wave over `sinLimit = 90 + (value - 10) * 9`.
//!
//! The renderer is [`render_ground_arc_glow`]
//! (`super::super::render_slots::render_ground_arc_glow`, official
//! `RenderGroundArcGlow`, `EffectPrimitiveArcs.cpp:60-166`).
//!
//! Quirk kept: `RenderGroundArcGlow` derives the ring radius from
//! `sin(slot.rotStart) + distance` instead of `distance`, so the arc breathes
//! as the slot turns.

use super::{EffectPrimitiveState, MasterState};
use crate::world::effect::port::angles::Angles;

/// One frame of `PrimWaveHeights` (`EffectPrimitiveMotion.cpp:5534-5583`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    // `FollowMaster(*this)` (`EffectPrimitiveMotion.cpp:115-119`).
    state.position = master.position;

    // `const int middle = (21 - 1) / 2; const int m2 = 90 / middle;`
    const MIDDLE: i32 = 10;
    const M2: i32 = 9;

    let duration = state.duration;

    for index in 0..4usize {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];

        slot.process += 1;

        if slot.process <= 0 {
            continue;
        }

        slot.rot_start += 3;

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        if slot.process < 30 {
            slot.alpha_b += 3;

            if slot.alpha_b > 100 {
                slot.alpha_b = 100;
            }
        }

        if slot.process > duration - 46 {
            slot.alpha_b -= 2;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }

        for value in 0..21usize {
            let pr = if index < 2 {
                (slot.process + index as i32 * 90) % 360
            } else {
                (slot.process * 2 + index as i32 * 90) % 360
            };

            let sin_limit = 90 + (value as i32 - MIDDLE) * M2;

            slot.height[value] = slot.max_height;
            // Kept operand-for-operand: `sin(sinLimit) * maxHeight * 0.3f *
            // sin(pr)`.
            slot.height[value] += angles.raw_sine(sin_limit as usize) * slot.max_height * 0.3 * angles.raw_sine(pr as usize);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::WaveHeights,
            duration: 120,
            ..Default::default()
        };

        for index in 0..4usize {
            state.slots[index] = EffectPrimitiveSlot {
                life: 1,
                process: 0,
                max_height: 23.0,
                rot_start: [180, 270, 0, 90][index],
                rise_angle: [75, 63, 52, 40][index],
                distance: 4.3,
                ..Default::default()
            };
        }

        state
    }

    #[test]
    fn every_slot_turns_three_degrees_and_ramps_to_one_hundred() {
        let angles = Angles::new();
        let mut state = state();

        on_process(&mut state, &angles, &MasterState::default());

        assert_eq!(state.slots[0].rot_start, 183);
        assert_eq!(state.slots[3].rot_start, 93);
        assert_eq!(state.slots[0].alpha_b, 3);
    }

    #[test]
    fn the_wave_rides_the_sine_envelope() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 4;

        on_process(&mut state, &angles, &MasterState::default());

        // process 5: value 10 is the middle sample, sin(90) = 1, so the height
        // is `23 + 23 * 0.3 * sin(5)`.
        let expected = 23.0 + 23.0 * 0.3 * angles.raw_sine(5usize);
        assert!((state.slots[0].height[10] - expected).abs() < 1e-3);
        // value 0 sits on `sin(0)`, so the wave does not lift it.
        assert!((state.slots[0].height[0] - 23.0).abs() < 1e-3);
    }

    #[test]
    fn the_tail_fades_two_per_frame() {
        let angles = Angles::new();
        let mut state = state();
        // duration 120 -> the fade starts when process > 74.
        state.slots[0].process = 74;
        state.slots[0].alpha_b = 100;

        on_process(&mut state, &angles, &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 98);
    }

    #[test]
    fn the_master_position_is_followed() {
        let angles = Angles::new();
        let mut state = state();
        let master = MasterState {
            position: [4.0, 5.0, 6.0],
            ..Default::default()
        };

        on_process(&mut state, &angles, &master);

        assert_eq!(state.position, [4.0, 5.0, 6.0]);
    }
}
