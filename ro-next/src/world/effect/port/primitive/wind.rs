//! `PP_WIND` - `CEffectPrim::PrimWind`, `EffectPrimitiveMotion.cpp:4415-4497`.
//!
//! Four slot-driven rising wind columns (`cloud11.tga` in `Cloud11Burst`). The
//! motion is pure slot-state arithmetic: rotation, display-angle growth, alpha
//! ramps and distance drift.

use super::EffectPrimitiveState;

/// One frame of `PrimWind`.
pub fn on_process(state: &mut EffectPrimitiveState) {
    for index in 0..4 {
        if state.slots[index].life != 1 {
            continue;
        }

        let slot = &mut state.slots[index];
        slot.process += 1;

        if slot.process > 0 {
            slot.rot_start += 5;

            if slot.rot_start >= 360 {
                slot.rot_start -= 360;
            }

            if slot.alpha_t == 2 {
                slot.full_display_angle += 3;

                if slot.full_display_angle > 180 {
                    slot.full_display_angle = 180;
                }
            } else {
                slot.full_display_angle += 3;

                if slot.full_display_angle > 120 {
                    slot.full_display_angle = 120;
                }
            }

            if slot.alpha_t == 1 {
                if slot.process > 20 {
                    slot.alpha_b -= 2;

                    if slot.alpha_b < 0 {
                        slot.alpha_b = 0;
                    }
                }

                if slot.process > 20 {
                    slot.distance += 0.1;
                }
            } else if slot.alpha_t == 2 {
                if slot.process > 50 {
                    slot.alpha_b -= 1;

                    if slot.alpha_b < 0 {
                        slot.alpha_b = 0;
                    }
                }

                if slot.process > 12 {
                    slot.distance += 0.1;
                }
            }

            if slot.process > 1400 {
                if slot.alpha_t != 3 {
                    slot.alpha_b -= 3;
                }

                if slot.alpha_b < 0 {
                    slot.alpha_b = 0;
                }
            }

            if slot.alpha_t == 2 {
                if slot.process < 12 {
                    slot.alpha_b += 1;
                }
            } else if slot.process < 12 {
                slot.alpha_b += 10;

                if slot.alpha_b > 250 {
                    slot.alpha_b = 250;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::EffectPrimitiveSlot;

    fn wind_state(alpha_t: i16) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: crate::world::effect::port::primitive::PrimitiveId::Wind,
            duration: 200,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            alpha_t,
            full_display_angle: 30,
            ..Default::default()
        };
        state
    }

    #[test]
    fn process_rotates_and_grows_the_display_angle() {
        let mut state = wind_state(1);

        on_process(&mut state);

        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].rot_start, 5);
        assert_eq!(state.slots[0].full_display_angle, 33);
        assert_eq!(state.slots[0].alpha_b, 10);
    }

    #[test]
    fn display_angle_clamps_at_120_or_180() {
        let mut state = wind_state(1);
        state.slots[0].full_display_angle = 119;
        on_process(&mut state);
        assert_eq!(state.slots[0].full_display_angle, 120);

        let mut state = wind_state(2);
        state.slots[0].full_display_angle = 179;
        on_process(&mut state);
        assert_eq!(state.slots[0].full_display_angle, 180);
    }

    #[test]
    fn inactive_slots_are_ignored() {
        let mut state = wind_state(1);
        state.slots[1].life = 0;
        state.slots[2].life = 0;
        state.slots[3].life = 0;

        on_process(&mut state);
        on_process(&mut state);

        assert_eq!(state.slots[1].process, 0);
        assert_eq!(state.slots[0].process, 2);
    }

    #[test]
    fn alpha_ramps_down_after_twenty_frames() {
        let mut state = wind_state(1);
        state.slots[0].process = 25;
        state.slots[0].alpha_b = 250;
        on_process(&mut state);
        assert!((state.slots[0].distance - 0.1).abs() < 1e-6);
        assert_eq!(state.slots[0].alpha_b, 248);
    }
}
