//! prims 130 (`PrimRisingFan`) and 131 (`PrimRisingFan2`) -
//! `EffectPrimitiveMotion.cpp:6314-6479`.
//!
//! Tall rising "fan" sheets (Saint Wing, ice wings). Both share
//! `RaiseFanHeights`: ten height samples with per-sample caps.

use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState};

/// `RaiseFanHeights(slot, step)` - `EffectPrimitiveMotion.cpp:6314-6374`.
fn raise_fan_heights(slot: &mut EffectPrimitiveSlot, step: f32) {
    const CAPS: [f32; 10] = [70.0, 90.0, 110.0, 130.0, 150.0, 170.0, 190.0, 210.0, 230.0, 250.0];

    for (index, cap) in CAPS.iter().enumerate() {
        let sample = index + 1;
        slot.height[sample] += step;

        if slot.height[sample] >= *cap {
            slot.height[sample] = *cap;
        }
    }
}

/// One frame of `PrimRisingFan`.
pub fn rising_fan(state: &mut EffectPrimitiveState, master: &MasterState) {
    if state.slots[0].flag1[4] == 1 {
        state.position = master.position;
    }

    for index in 0..4 {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];
        slot.process += 1;

        if slot.process <= 0 {
            continue;
        }

        if slot.process < 20 {
            raise_fan_heights(slot, 10.0);
        } else {
            for step in 1..10 {
                slot.height[step] -= 4.0;

                if slot.height[step] < 0.0 {
                    slot.height[step] = 0.0;
                }
            }

            if (slot.flag1[4] == 6 || slot.flag1[4] == 7) && slot.process > 90 {
                slot.height[10] -= 5.0;

                if slot.height[10] > 0.0 {
                    slot.distance *= 1.01;
                }

                if slot.height[10] < 0.0 {
                    slot.height[10] = 0.0;
                }
            }
        }

        if slot.process < 50 {
            slot.distance -= 0.2;
        }
    }
}

/// One frame of `PrimRisingFan2`.
pub fn rising_fan2(state: &mut EffectPrimitiveState, master: &MasterState) {
    if state.slots[0].flag1[4] == 1 {
        state.position = master.position;
    }

    for index in 0..4 {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];
        slot.process += 1;

        if slot.process <= 0 {
            continue;
        }

        if slot.process < 20 {
            raise_fan_heights(slot, 13.0);
        } else {
            for step in 1..10 {
                slot.height[step] -= 5.0;

                if slot.height[step] < 0.0 {
                    slot.height[step] = 0.0;
                }
            }

            if slot.process > 100 {
                slot.height[10] -= 5.0;

                if slot.height[10] > 0.0 {
                    slot.distance *= 1.01;
                }

                if slot.height[10] < 0.0 {
                    slot.height[10] = 0.0;
                }
            }
        }

        slot.distance *= 1.01;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn fan(count: PrimitiveId, flag: i8) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count,
            duration: 300,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            distance: 10.0,
            flag1: {
                let mut flag1 = [0i8; 21];
                flag1[4] = flag;
                flag1
            },
            ..Default::default()
        };
        state
    }

    #[test]
    fn rising_fan_raises_heights_with_caps() {
        let master = MasterState::default();
        let mut state = fan(PrimitiveId::RisingFan, 0);

        rising_fan(&mut state, &master);

        assert_eq!(state.slots[0].height[1], 10.0);
        assert_eq!(state.slots[0].height[10], 10.0);
        assert!((state.slots[0].distance - 9.8).abs() < 1e-6, "distance closes first");
    }

    #[test]
    fn rising_fan_caps_at_the_sample_limit() {
        let master = MasterState::default();
        let mut state = fan(PrimitiveId::RisingFan, 0);
        state.slots[0].process = 6;
        state.slots[0].height[1] = 68.0;

        rising_fan(&mut state, &master);

        assert_eq!(state.slots[0].height[1], 70.0);
    }

    #[test]
    fn rising_fan_two_uses_the_larger_step() {
        let master = MasterState::default();
        let mut state = fan(PrimitiveId::RisingFan2, 1);

        rising_fan2(&mut state, &master);

        assert_eq!(state.slots[0].height[1], 13.0);
        assert!((state.slots[0].distance - 10.1).abs() < 1e-4, "second fan only grows");
    }

    #[test]
    fn rising_fan_two_fades_after_twenty_frames() {
        let master = MasterState::default();
        let mut state = fan(PrimitiveId::RisingFan2, 1);
        state.slots[0].process = 25;
        state.slots[0].height[5] = 100.0;

        rising_fan2(&mut state, &master);

        assert_eq!(state.slots[0].height[5], 95.0);
    }
}
