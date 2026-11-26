//! prim 63 (`PrimRippleRings`) - `EffectPrimitiveMotion.cpp:6638-6739`.
//!
//! Three expanding rings controlled by a fourth "driver" slot (`slots[3]`):
//! the driver's `alphaT` selects the ring behaviour (3 = contracting, 4 = soft
//! fade, 5 = bright), its `process` counts the lifetime.

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimRippleRings`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    state.position = master.position;

    for index in 0..3usize {
        if state.slots[index].life != 1 {
            continue;
        }

        state.slots[index].process += 1;

        if index == 0 {
            state.slots[3].process += 1;
        }

        if state.slots[index].process <= 0 {
            continue;
        }

        if state.slots[3].alpha_t == 3 {
            state.slots[index].distance -= 0.25;

            if state.slots[index].distance < 7.0 {
                if state.slots[index].distance < 0.0 {
                    state.slots[index].distance = 0.0;
                }

                state.slots[index].alpha_b -= 7;

                if state.slots[index].alpha_b < 0 {
                    state.slots[index].alpha_b = 0;

                    if state.slots[3].process > 1400 {
                        state.slots[index].life = 0;
                    } else {
                        state.slots[index].process = 0;
                        state.slots[index].distance = 14.0;
                    }
                }
            }
        } else {
            state.slots[index].distance += 0.5;

            if state.slots[index].distance > 7.0 {
                if state.slots[3].alpha_t == 4 {
                    state.slots[index].alpha_b -= 5;
                } else if state.slots[3].alpha_t == 5 {
                    state.slots[index].alpha_b -= 8;
                } else {
                    state.slots[index].alpha_b -= 15;
                }

                if state.slots[index].alpha_b < 0 {
                    state.slots[index].alpha_b = 0;

                    let driver = state.slots[3];
                    let fade_out = (driver.process > 120
                        && (driver.alpha_t == 1 || driver.alpha_t == 4 || driver.alpha_t == 5))
                        || driver.process > 1400;

                    if fade_out {
                        state.slots[index].life = 0;
                    } else {
                        state.slots[index].process = 0;
                        state.slots[index].distance = 0.0;
                    }
                }
            }
        }

        if state.slots[3].alpha_t != 3 {
            if state.slots[index].process < 10 {
                if state.slots[3].alpha_t == 4 {
                    state.slots[index].alpha_b += 4;

                    if state.slots[index].alpha_b > 40 {
                        state.slots[index].alpha_b = 40;
                    }
                } else {
                    state.slots[index].alpha_b += 24;

                    if state.slots[3].alpha_t == 5 {
                        if state.slots[index].alpha_b > 120 {
                            state.slots[index].alpha_b = 120;
                        }
                    } else if state.slots[index].alpha_b > 240 {
                        state.slots[index].alpha_b = 240;
                    }
                }
            }
        } else if state.slots[index].process < 20 {
            state.slots[index].alpha_b += 12;

            if state.slots[index].alpha_b > 240 {
                state.slots[index].alpha_b = 240;
            }
        }

        for column in 0..21usize {
            let mut height = state.slots[index].max_height;

            if state.slots[index].process <= 10 {
                height *= angles.raw_sine((state.slots[index].process * 9).max(0) as usize);
            }

            state.slots[index].height[column] = height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn rings(driver_alpha_t: i16) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::RippleRings,
            duration: 300,
            ..Default::default()
        };

        for index in 0..3 {
            state.slots[index] = EffectPrimitiveSlot {
                life: 1,
                distance: 0.0,
                max_height: 40.0,
                ..Default::default()
            };
        }

        state.slots[3].life = 1;
        state.slots[3].alpha_t = driver_alpha_t;
        state
    }

    #[test]
    fn expanding_rings_gain_distance_and_alpha() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = rings(1);

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].distance, 0.5);
        assert_eq!(state.slots[0].alpha_b, 24);
        assert_eq!(state.slots[3].process, 1);
    }

    #[test]
    fn contracting_rings_pull_inward() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = rings(3);
        state.slots[0].distance = 10.0;

        on_process(&mut state, &angles, &master);

        assert!((state.slots[0].distance - 9.75).abs() < 1e-6);
        assert_eq!(state.slots[0].alpha_b, 12);
    }

    #[test]
    fn soft_driver_caps_alpha_lower() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = rings(4);
        state.slots[0].process = 0;

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].alpha_b, 4);
    }

    #[test]
    fn heights_use_the_sine_ramp_in_the_first_ten_frames() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = rings(1);

        on_process(&mut state, &angles, &master);

        let expected = 40.0 * angles.raw_sine(9);
        assert!((state.slots[0].height[0] - expected).abs() < 1e-4);
    }
}
