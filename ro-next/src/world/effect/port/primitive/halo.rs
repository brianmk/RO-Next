//! prim 39 (`TexturedHalo` primitive) - `EffectPrimitiveMotion.cpp:9651-9688`.
//!
//! A persistent ground halo: two slots whose rotation, orbit angle and distance
//! react to `height[4]` (the halo `kind`).

use super::{EffectPrimitiveState, MasterState};

/// One frame of the halo primitive.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    state.position = master.position;

    for index in 0..2usize {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];

        if slot.height[4] == 4.0 {
            slot.rot_start += 6;

            if slot.rot_start >= 360 {
                slot.rot_start -= 360;
            }
        } else if slot.height[4] == 10.0 || slot.height[4] == 11.0 || slot.height[4] == 12.0 {
            slot.rot_start += 3;

            if slot.rot_start >= 360 {
                slot.rot_start -= 360;
            }
        }

        if slot.height[4] == 2.0 {
            slot.rise_angle += 1;
        } else if slot.height[4] == 10.0 {
            if slot.rot_start < 181 {
                slot.distance -= 0.3;
            } else {
                slot.distance += 0.3;
            }
        } else if slot.height[4] != 11.0 && slot.height[4] != 12.0 {
            slot.rise_angle += 3;
        }

        if slot.rise_angle >= 360 {
            slot.rise_angle -= 360;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn halo(kind: f32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Halo,
            duration: 29999,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            distance: 10.0,
            height: {
                let mut height = [0.0f32; 21];
                height[4] = kind;
                height
            },
            ..Default::default()
        };
        state
    }

    #[test]
    fn kind_two_orbits_one_degree_per_frame() {
        let master = MasterState::default();
        let mut state = halo(2.0);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].rise_angle, 1);
        assert_eq!(state.slots[0].rot_start, 0);
    }

    #[test]
    fn kind_four_spins_faster() {
        let master = MasterState::default();
        let mut state = halo(4.0);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].rot_start, 6);
        assert_eq!(state.slots[0].rise_angle, 3);
    }

    #[test]
    fn kind_ten_pulses_the_distance() {
        let master = MasterState::default();
        let mut state = halo(10.0);
        state.slots[0].rot_start = 10;

        on_process(&mut state, &master);
        assert!((state.slots[0].distance - 9.7).abs() < 1e-5);

        state.slots[0].rot_start = 180;
        on_process(&mut state, &master);
        assert!((state.slots[0].distance - 10.0).abs() < 1e-5);
    }

    #[test]
    fn kind_eleven_holds_still() {
        let master = MasterState::default();
        let mut state = halo(11.0);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].rise_angle, 0);
        assert!((state.slots[0].distance - 10.0).abs() < 1e-6);
    }
}
