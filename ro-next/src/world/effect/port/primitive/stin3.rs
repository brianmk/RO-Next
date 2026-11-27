//! prim 122 (no `EffectPrimId.h` symbol) - `CEffectPrim::PrimSTIN3`,
//! `EffectPrimitiveMotion.cpp:7071-7173`; dispatch `case 122:`
//! `EffectPrimitiveMotion.cpp:9451-9452`.
//!
//! The two wind crescents of `CRagEffect::TexturedSlashLevel` (`EF_STIN2`).
//! Only slot 0 is stepped: it turns toward the target on frame 1 (plus/minus
//! 90 degrees by `flag1[2]`), fades in to 250 until process 20, and switches
//! `flag1[1]` to the fade-out state after process 140 (90 for
//! `flag1[4] == 1`). While homing (`flag1[1] == 0`) it turns 5 degrees per
//! frame toward the target; in the fade state it bleeds all four slots' alpha
//! by 5 and spins one degree; state 20 (target lost) is kept verbatim even
//! though the port always resolves a target position.
//!
//! Like [`super::line_link`], the original's `g_session.m_aid` /
//! `GetGameActorByAID` lookup is replaced by the spawn bridge's resolved target
//! position in `master.master_delta_pos` (a spawn-time snapshot). The renderer
//! is [`render_floor_square2`](super::super::render_slots::render_floor_square2)
//! (`RenderFloorSquare2`, dispatch `EffectPrimitiveRendering.cpp:549-551`).

use super::{Angles, EffectPrimitiveState, MasterState, get_angle};

/// One frame of `PrimSTIN3` (`EffectPrimitiveMotion.cpp:7071-7173`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    let target = master.master_delta_pos;

    if state.slots[0].life != 1 {
        return;
    }

    state.slots[0].process += 1;

    if state.slots[0].process <= 0 {
        return;
    }

    {
        let slot = &mut state.slots[0];

        slot.rise_angle += 30;

        if slot.rise_angle >= 360 {
            slot.rise_angle -= 360;
        }

        if slot.process == 1 {
            slot.rot_start = get_angle(target[0] - state.position[0], target[2] - state.position[2]);

            if slot.flag1[2] == 0 {
                slot.rot_start += 90;

                if slot.rot_start >= 360 {
                    slot.rot_start -= 360;
                }
            } else {
                slot.rot_start -= 90;

                if slot.rot_start < 0 {
                    slot.rot_start += 360;
                }
            }
        } else if slot.process <= 20 {
            slot.alpha_b += 15;

            if slot.alpha_b > 250 {
                slot.alpha_b = 250;
            }
        } else if slot.process > 140 || (slot.process > 90 && slot.flag1[4] == 1) {
            slot.flag1[1] = 10;
        }
    }

    if state.slots[0].process > 5 {
        match state.slots[0].flag1[1] {
            0 => {
                let slot = &mut state.slots[0];
                let heading = get_angle(target[0] - slot.vec_b_now[0], target[2] - slot.vec_b_now[2]);

                if slot.rot_start < heading - 3 || slot.rot_start > heading + 3 {
                    if slot.flag1[2] == 0 {
                        slot.rot_start -= 5;

                        if slot.rot_start < 0 {
                            slot.rot_start += 360;
                        }
                    } else {
                        slot.rot_start += 5;

                        if slot.rot_start >= 360 {
                            slot.rot_start -= 360;
                        }
                    }
                }
            }
            10 => {
                for fade in 0..4usize {
                    state.slots[fade].alpha_b -= 5;

                    if state.slots[fade].alpha_b < 0 {
                        state.slots[fade].alpha_b = 0;
                        state.slots[fade].life = 0;
                    }
                }

                let slot = &mut state.slots[0];

                if slot.flag1[2] == 0 {
                    slot.rot_start -= 1;

                    if slot.rot_start < 0 {
                        slot.rot_start += 360;
                    }
                } else {
                    slot.rot_start += 1;

                    if slot.rot_start >= 360 {
                        slot.rot_start -= 360;
                    }
                }
            }
            20 => {
                let slot = &mut state.slots[0];
                let heading = get_angle(state.position[0] - slot.vec_b_now[0], state.position[2] - slot.vec_b_now[2]);

                if slot.rot_start < heading - 3 || slot.rot_start > heading + 3 {
                    if slot.flag1[2] == 0 {
                        slot.rot_start -= 5;

                        if slot.rot_start < 0 {
                            slot.rot_start += 360;
                        }
                    } else {
                        slot.rot_start += 5;

                        if slot.rot_start >= 360 {
                            slot.rot_start -= 360;
                        }
                    }
                }

                if slot.process > 50 {
                    slot.flag1[1] = 10;
                }
            }
            _ => {}
        }
    }

    {
        let slot = &mut state.slots[0];
        slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start as usize);
        slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start as usize);
    }

    if state.slots[0].process % 3 == 0 {
        for trail in (0..3usize).rev() {
            state.slots[trail + 1].vec_b_now = state.slots[trail].vec_b_now;
            state.slots[trail + 1].alpha_b = (state.slots[trail].alpha_b as f32 * 0.8) as i16;
            state.slots[trail + 1].rise_angle = state.slots[trail].rise_angle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Stin3,
            duration: 200,
            position: [0.0, 0.0, 0.0],
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: 0,
            height: [0.0; 21],
            ..Default::default()
        };
        state.slots[0].height[1] = 1.0;
        state
    }

    #[test]
    fn first_frame_turns_ninety_degrees_off_the_target() {
        let angles = Angles::new();
        let master = MasterState {
            master_delta_pos: [0.0, 0.0, 10.0],
            ..Default::default()
        };
        let mut state = state();

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].process, 1);
        // Get_Angle gives 91, +90 = 181.
        assert_eq!(state.slots[0].rot_start, 181);
        assert_eq!(state.slots[0].rise_angle, 30);
    }

    #[test]
    fn fades_in_until_twenty() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 10;
        state.slots[0].alpha_b = 240;

        on_process(&mut state, &angles, &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 250);

        state.slots[0].alpha_b = 250;
        on_process(&mut state, &angles, &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 250);
    }

    #[test]
    fn the_short_lived_variant_fades_after_ninety() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 90;
        state.slots[0].flag1[4] = 1;
        state.slots[0].alpha_b = 100;

        on_process(&mut state, &angles, &MasterState::default());

        assert_eq!(state.slots[0].flag1[1], 10);
        // The fade branch runs the same frame: every slot loses five.
        assert_eq!(state.slots[0].alpha_b, 95);
        assert_eq!(state.slots[1].alpha_b, 0);
    }
}
