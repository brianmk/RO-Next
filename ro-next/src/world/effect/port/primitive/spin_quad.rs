//! prim 89 (`TexturedQuadSpin` primitive) - the `case 89` block of
//! `EffectPrimitiveMotion.cpp:9817-9859`.
//!
//! Four spinning quads: follow the master, ramp alpha and push the slot
//! distance outward (or scale it by 1.15) depending on `height[4]`.

use super::{EffectPrimitiveState, MasterState};

/// One frame of the spin-quad primitive.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    state.position = master.position;

    for index in 0..4 {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];
        let process = slot.process + 1;
        slot.process = process;

        if process <= 0 {
            continue;
        }

        let kind = slot.height[4];

        if kind == 2.0 || kind == 6.0 {
            if process <= 10 {
                slot.alpha_b += 10;
            } else if process > 11 {
                slot.alpha_b -= 3;

                if slot.alpha_b < 0 {
                    slot.alpha_b = 0;
                }
            }

            slot.distance += 3.0;
        } else if kind == 3.0 {
            if process <= 10 {
                slot.alpha_b += 6;
            } else if process > 11 {
                slot.alpha_b -= 1;

                if slot.alpha_b < 0 {
                    slot.alpha_b = 0;
                }
            }

            slot.distance += 3.0;
        } else {
            if process <= 10 {
                slot.alpha_b += 20;
            } else if process > 12 {
                slot.alpha_b -= 15;

                if slot.alpha_b < 0 {
                    slot.alpha_b = 0;
                }
            }

            slot.distance *= 1.15;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn quad(kind: f32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::SpinQuad,
            duration: 200,
            ..Default::default()
        };
        let mut height = [0.0f32; 21];
        height[4] = kind;
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            distance: 2.0,
            height,
            ..Default::default()
        };
        state
    }

    #[test]
    fn kind_two_ramps_alpha_and_pushes_outward() {
        let master = MasterState::default();
        let mut state = quad(2.0);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].alpha_b, 10);
        assert_eq!(state.slots[0].distance, 5.0);
    }

    #[test]
    fn kind_three_ramps_slower() {
        let master = MasterState::default();
        let mut state = quad(3.0);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].alpha_b, 6);
        assert_eq!(state.slots[0].distance, 5.0);
    }

    #[test]
    fn other_kinds_scale_the_distance() {
        let master = MasterState::default();
        let mut state = quad(5.0);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].alpha_b, 20);
        assert!((state.slots[0].distance - 2.3).abs() < 1e-6);
    }

    #[test]
    fn late_frames_fade_out() {
        let master = MasterState::default();
        let mut state = quad(2.0);
        state.slots[0].process = 20;
        state.slots[0].alpha_b = 100;

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].alpha_b, 97);
    }
}
