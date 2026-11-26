//! Prim 82 (`PrimAuraGlow`) - `EffectPrimitiveMotion.cpp:6837-6946`.
//!
//! The persistent aura primitive behind `TexturedAura` (glows, status overlays,
//! glows on maps). One active slot with a per-kind alpha ramp and orbit angle.

use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimAuraGlow`.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    state.position = master.position;

    let slot = &mut state.slots[0];

    if slot.life != 1 {
        return;
    }

    slot.process += 1;

    if slot.process < 300 {
        let kind = slot.flag1[4];

        if kind == 1 || kind == 3 {
            if slot.process < 200 {
                slot.alpha_b += 10;

                if slot.alpha_b >= 220 {
                    slot.alpha_b = 220;
                }
            } else {
                slot.alpha_b -= 5;

                if slot.alpha_b < 0 {
                    slot.alpha_b = 0;
                }
            }
        } else if kind == 22 {
            slot.alpha_b += 10;

            if slot.alpha_b >= 220 {
                slot.alpha_b = 220;
            }
        } else if kind == 0 {
            slot.alpha_b += 1;
        } else if kind == 20 {
            slot.alpha_b += 5;

            if slot.alpha_b >= 200 {
                slot.alpha_b = 200;
            }
        } else if (16..=18).contains(&kind) {
            slot.alpha_b = 100;
        } else if (10..=12).contains(&kind) {
            slot.alpha_b = 150;
        } else if kind == 14 {
            slot.alpha_b = 50;
        } else if kind >= 10 {
            slot.alpha_b = 250;
        }
    }

    let kind = slot.flag1[4];

    if kind == 2 {
        if slot.process > 40 {
            let alpha = slot.alpha_b as i16 + if slot.process >= 62 { -8 } else { 8 };
            slot.alpha_b = alpha;

            if alpha > 0 {
                slot.distance += 0.7;
            }
        }

        if slot.process > 300 && slot.alpha_b > 0 {
            slot.alpha_b -= 1;
        }
    } else if kind == 16 || kind == 17 {
        if (slot.process & 1) == 0 {
            slot.full_display_angle -= 1;
        }

        if slot.full_display_angle < 0 {
            slot.full_display_angle += 360;
        }
    } else if kind < 20 {
        if kind == 18 {
            if slot.full_display_angle < 0 {
                slot.full_display_angle += 360;
            }
        } else if kind == 10 || kind >= 13 {
            if (slot.process & 1) == 0 {
                slot.full_display_angle += 1;
            }

            if slot.full_display_angle >= 360 {
                slot.full_display_angle -= 360;
            }
        } else if kind == 11 || kind == 12 {
            if (slot.process & 1) == 0 {
                slot.full_display_angle -= 1;
            }

            if slot.full_display_angle < 0 {
                slot.full_display_angle += 360;
            }
        } else if slot.process > 300 && slot.alpha_b > 0 {
            slot.alpha_b -= 1;
        }
    }

    if slot.alpha_b >= 255 {
        slot.alpha_b = 255;
    } else if slot.alpha_b < 0 {
        slot.alpha_b = 0;
    }

    if slot.flag1[4] < 30 {
        slot.height[8] += 2.0;
    }

    if slot.height[8] >= 360.0 {
        slot.height[8] -= 360.0;
    }

    slot.vec_b_now[1] = state.position[1] - slot.height[15];
    slot.vec_b_now[0] = state.position[0];
    slot.vec_b_now[2] = state.position[2];
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn aura(kind: i8) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::AuraGlow,
            duration: 1000,
            ..Default::default()
        };
        let mut flag1 = [0i8; 21];
        flag1[4] = kind;
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            flag1,
            ..Default::default()
        };
        state
    }

    #[test]
    fn glow_alpha_ramps_and_clamps() {
        let master = MasterState::default();
        let mut state = aura(1);
        state.slots[0].process = 0;
        state.slots[0].alpha_b = 215;

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].alpha_b, 220);
    }

    #[test]
    fn kind_ten_rotates_once_per_two_frames() {
        let master = MasterState::default();
        let mut state = aura(10);
        state.slots[0].process = 1;

        on_process(&mut state, &master);
        assert_eq!(state.slots[0].full_display_angle, 1);

        on_process(&mut state, &master);
        assert_eq!(state.slots[0].full_display_angle, 1, "odd process step stays");

        on_process(&mut state, &master);
        assert_eq!(state.slots[0].full_display_angle, 2);
    }

    #[test]
    fn kind_twenty_two_sets_the_status_alpha() {
        let master = MasterState::default();
        let mut state = aura(22);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].alpha_b, 10);
    }

    #[test]
    fn position_follows_the_master() {
        let master = MasterState {
            position: [1.0, 2.0, 3.0],
            ..Default::default()
        };
        let mut state = aura(0);

        on_process(&mut state, &master);

        assert_eq!(state.position, [1.0, 2.0, 3.0]);
        assert_eq!(state.slots[0].vec_b_now, [1.0, 2.0 - state.slots[0].height[15], 3.0]);
    }
}
