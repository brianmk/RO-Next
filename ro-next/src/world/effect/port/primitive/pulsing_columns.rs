//! prim 60 (`PrimPulsingColumns`, also `PP_HEAL_LOW`) -
//! `EffectPrimitiveMotion.cpp:6541-6650`.
//!
//! The pulsing heal/fire columns: 21 height samples per slot driven by a sine
//! phase, with per-slot-kind rotation speeds and alpha ramps.

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimPulsingColumns`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    if state.slots[0].flag1[0] == 33 {
        // `360 - camera->camera.yaw`
        state.slots[2].alpha_t = (360.0 - master.camera_yaw) as i16;
    }

    state.position = master.position;

    for index in 0..4usize {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];
        slot.process += 1;
        let process = slot.process;

        match slot.flag1[0] {
            1 => slot.rot_start += index as i16 + 8,
            2 => slot.rot_start += if index < 2 { index as i16 + 4 } else { index as i16 + 2 },
            3 => slot.rot_start += index as i16 + 6,
            _ => {}
        }

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        if slot.process < slot.alpha_t as i32 || slot.alpha_t < 0 {
            if slot.process < 16 {
                match slot.flag1[0] {
                    2 => slot.alpha_b += 3,
                    3 => slot.alpha_b += 2,
                    _ => slot.alpha_b += 5,
                }

                if slot.alpha_b > 180 {
                    slot.alpha_b = 180;
                }
            }
        } else {
            slot.alpha_b -= 2;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }

        for column in 0..21usize {
            let kind = slot.flag1[column];

            if kind == 0 || kind == 33 {
                let phase = if index == 0 {
                    column as i32 * 34 + process * 4
                } else if index == 1 {
                    column as i32 * 34 + process * 3
                } else {
                    (process + column as i32 * 17) * 2
                };

                let mut height = slot.max_height * 0.25 * angles.raw_sine((phase.rem_euclid(360)) as usize)
                    + slot.max_height * 0.75;

                if slot.process <= 90 {
                    height *= angles.raw_sine(slot.process.max(0) as usize);
                }

                slot.height[column] = height.max(0.0);
            } else if kind == 1 || kind == 3 {
                let mut height = slot.max_height;

                if slot.process <= 90 {
                    height *= angles.raw_sine(slot.process.max(0) as usize);
                }

                slot.height[column] = height;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn columns() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::PulsingColumns,
            duration: 200,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            max_height: 100.0,
            flag1: {
                let mut flag1 = [0i8; 21];
                flag1[0] = 1;
                flag1
            },
            ..Default::default()
        };
        state
    }

    #[test]
    fn alpha_ramps_in_the_first_frames() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = columns();
        // Ramp only runs while `process < alphaT` (or alphaT is negative).
        state.slots[0].alpha_t = -1;

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].alpha_b, 5);
        assert_eq!(state.slots[0].process, 1);
    }

    #[test]
    fn rotated_start_advances_by_kind() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = columns();

        on_process(&mut state, &angles, &master);

        // kind 1 at index 0: +8 per frame.
        assert_eq!(state.slots[0].rot_start, 8);
    }

    #[test]
    fn flat_columns_use_max_height_then_sine() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = columns();
        state.slots[0].process = 50;

        on_process(&mut state, &angles, &master);

        // kind 1 with process <= 90: maxHeight * sin(process).
        let expected = 100.0 * angles.raw_sine(51);
        assert!((state.slots[0].height[0] - expected).abs() < 1e-4);
    }

    #[test]
    fn camera_yaw_sets_the_special_slot() {
        let angles = Angles::new();
        let master = MasterState {
            camera_yaw: 90.0,
            ..Default::default()
        };
        let mut state = columns();
        state.slots[0].flag1[0] = 33;

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[2].alpha_t, 270);
    }
}
