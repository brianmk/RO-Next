//! prim 125 (`PP_STIN4`) - `CEffectPrim::PrimSTIN4`,
//! `EffectPrimitiveMotion.cpp:3874-3977`; dispatch `case PP_STIN4:`
//! `EffectPrimitiveMotion.cpp:9454-9455`.
//!
//! The four wind slices of `CRagEffect::TexturedSlashWave` (`EF_STIN4`). Only
//! slot 0 is stepped: frame 1 turns toward the target and adds
//! `rand(21) + 25` degrees (subtracts when `flag1[2]` is set), the next 20
//! frames raise `alphaB` to 120, then `flag1[1]` goes to 10. While homing the
//! slot turns 5 degrees toward the target and snaps to the fade state within
//! three degrees; the fade state bleeds every slot's alpha by 5 and spins one
//! degree per frame.
//!
//! Target resolution is the spawn bridge's snapshot in
//! `master.master_delta_pos` (the original queries `g_session.m_aid` /
//! `GetGameActorByAID`). Renderer:
//! [`render_floor_square2`](super::super::render_slots::render_floor_square2).

use super::{Angles, EffectPrimitiveState, MasterState, get_angle};

/// One frame of `PrimSTIN4` (`EffectPrimitiveMotion.cpp:3874-3977`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut super::EffectRng) {
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
                slot.rot_start += (rng.clamped_random(21) + 25) as i16;

                if slot.rot_start >= 360 {
                    slot.rot_start -= 360;
                }
            } else {
                slot.rot_start -= (rng.clamped_random(21) + 25) as i16;

                if slot.rot_start < 0 {
                    slot.rot_start += 360;
                }
            }
        } else if slot.process <= 20 {
            slot.alpha_b += 8;

            if slot.alpha_b > 120 {
                slot.alpha_b = 120;
            }
        } else {
            slot.flag1[1] = 10;
        }
    }

    if state.slots[0].process > 5 {
        match state.slots[0].flag1[1] {
            0 => {
                let slot = &mut state.slots[0];
                let heading = get_angle(target[0] - slot.vec_b_now[0], target[2] - slot.vec_b_now[2]);

                if slot.rot_start >= heading - 3 && slot.rot_start <= heading + 3 {
                    slot.flag1[1] = 10;
                } else if slot.flag1[2] == 0 {
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
            _ => {}
        }
    }

    let slot = &mut state.slots[0];
    slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start as usize);
    slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start as usize);

    if slot.process % 3 == 0 {
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
            count: PrimitiveId::Stin4,
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
    fn first_frame_aims_and_jitters_by_25_to_45_degrees() {
        let angles = Angles::new();
        let mut rng = super::super::EffectRng::new(1);
        let master = MasterState {
            master_delta_pos: [0.0, 0.0, 10.0],
            ..Default::default()
        };
        let mut state = state();

        on_process(&mut state, &angles, &master, &mut rng);

        assert_eq!(state.slots[0].process, 1);
        assert!((116..=136).contains(&state.slots[0].rot_start));
    }

    #[test]
    fn fades_in_until_twenty_then_sets_the_fade_flag() {
        let angles = Angles::new();
        let mut rng = super::super::EffectRng::new(1);
        let mut state = state();
        state.slots[0].process = 19;
        state.slots[0].alpha_b = 120;

        on_process(&mut state, &angles, &MasterState::default(), &mut rng);
        assert_eq!(state.slots[0].alpha_b, 120);

        state.slots[0].process = 20;
        on_process(&mut state, &angles, &MasterState::default(), &mut rng);
        assert_eq!(state.slots[0].flag1[1], 10);
        assert_eq!(state.slots[0].alpha_b, 115);
    }
}
