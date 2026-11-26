//! `PP_INTIMIDATE` - `CEffectPrim::PrimIntimidate`,
//! `EffectPrimitiveMotion.cpp:1257-1313`.
//!
//! Four slot-driven spray sprites (white01/white03) used by the Intimidate
//! family. Pure slot arithmetic: distance drift by `height[0]`, alpha ramp in
//! the first ten processed frames, fade after 25, rotation and orbit angle.
//!
//! `FaceCamera(slots)` only fills render-only orientation vectors and is not
//! needed for the motion; it will be ported with the renderer.

use super::EffectPrimitiveState;

/// One frame of `PrimIntimidate`.
pub fn on_process(state: &mut EffectPrimitiveState) {
    for index in 0..4 {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];
        slot.process += 1;

        if slot.process > 0 {
            slot.distance += slot.height[0];
        }

        if slot.process <= 10 && slot.process > 0 {
            if slot.flag1[0] == 0 {
                slot.alpha_b += 15;
            } else {
                slot.alpha_b += 25;
            }
        } else if slot.process > 25 {
            slot.alpha_b -= 7;

            if slot.alpha_b <= 0 {
                slot.alpha_b = 0;
            }
        }

        if slot.flag1[0] == 2 {
            slot.rot_start += 3;
        }

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        if slot.flag1[0] == 2 {
            slot.height[2] += 3.0;
        } else {
            slot.height[2] += 5.0;
        }

        if slot.height[2] >= 360.0 {
            slot.height[2] -= 360.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn spray_state(flag: i8) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Intimidate,
            duration: 999,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: -5,
            distance: 12.0,
            height: {
                let mut height = [0.0; 21];
                height[0] = 1.5;
                height
            },
            flag1: {
                let mut flag1 = [0; 21];
                flag1[0] = flag;
                flag1
            },
            ..Default::default()
        };
        state
    }

    #[test]
    fn distance_and_alpha_advance() {
        let mut state = spray_state(1);
        state.slots[0].process = 0;

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 1);
        assert!((state.slots[0].distance - 13.5).abs() < 1e-6);
        assert_eq!(state.slots[0].alpha_b, 25, "flag1 != 0 ramps by 25");
    }

    #[test]
    fn flag_zero_ramps_alpha_by_fifteen() {
        let mut state = spray_state(0);
        state.slots[0].process = 0;

        on_process(&mut state);

        assert_eq!(state.slots[0].alpha_b, 15);
    }

    #[test]
    fn fade_starts_after_twenty_five_frames() {
        let mut state = spray_state(0);
        state.slots[0].process = 30;
        state.slots[0].alpha_b = 100;

        on_process(&mut state);

        assert_eq!(state.slots[0].alpha_b, 93);
    }

    #[test]
    fn flag_two_spins_and_moves_slower() {
        let mut state = spray_state(2);
        state.slots[0].process = 5;
        state.slots[0].rot_start = 10;
        state.slots[0].height[2] = 100.0;

        on_process(&mut state);

        assert_eq!(state.slots[0].rot_start, 13);
        assert!((state.slots[0].height[2] - 103.0).abs() < 1e-6);
    }
}
