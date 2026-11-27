//! prim 119 (no `EffectPrimId.h` symbol) - the inline `case 119:`
//! (`EffectPrimitiveMotion.cpp:9400-9448`).
//!
//! The lead ring of `CRagEffect::TexturedSlashFan` (`EF_STIN`). Only slot 0
//! moves: it turns `rotStart` toward the target on the first frame, grows
//! `height[1]` and fades `alphaB` in for ten frames / out afterwards, then
//! copies its old position, alpha (x0.8) and rise angle into slots 1..3 every
//! third frame.
//!
//! Renderer:
//! [`render_floor_square2`](super::super::render_slots::render_floor_square2)
//! (`RenderFloorSquare2`, `EffectPrimitiveRendering.cpp:2674-2709`, dispatch
//! `:549-551`).

use super::{Angles, EffectPrimitiveState, MasterState, get_angle};

/// One frame of the inline `case 119` (`EffectPrimitiveMotion.cpp:9400-9448`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    if state.slots[0].life != 1 {
        return;
    }

    state.slots[0].process += 1;

    if state.slots[0].process <= 0 {
        return;
    }

    let target = master.master_delta_pos;

    {
        let slot = &mut state.slots[0];

        slot.rise_angle += 30;

        if slot.rise_angle > 359 {
            slot.rise_angle -= 360;
        }

        if slot.process == 1 {
            slot.rot_start = get_angle(target[0] - state.position[0], target[2] - state.position[2]);
        } else if slot.process < 11 {
            slot.alpha_b += 25;

            if slot.alpha_b > 250 {
                slot.alpha_b = 250;
            }

            slot.height[1] *= 1.02;
        } else {
            slot.alpha_b -= 5;
            slot.height[1] *= 1.04;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
                slot.life = 0;
            }
        }

        slot.vec_b_now[0] = slot.height[1] * angles.raw_cosine(slot.rot_start as usize) + slot.vec_b_now[0];
        slot.vec_b_now[2] = slot.height[1] * angles.raw_sine(slot.rot_start as usize) + slot.vec_b_now[2];
    }

    if state.slots[0].process % 3 == 0 {
        for index in (1..4usize).rev() {
            state.slots[index].vec_b_now = state.slots[index - 1].vec_b_now;
            state.slots[index].alpha_b = (state.slots[index - 1].alpha_b as f32 * 0.8) as i16;
            state.slots[index].rise_angle = state.slots[index - 1].rise_angle;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::EffectPrimitiveSlot;

    fn state(process: i32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: super::super::PrimitiveId::SlashFan,
            duration: 200,
            position: [0.0, 0.0, 0.0],
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process,
            height: [0.0; 21],
            ..Default::default()
        };
        state
    }

    #[test]
    fn first_frame_aims_at_the_target_and_grows() {
        let angles = Angles::new();
        let master = MasterState {
            master_delta_pos: [0.0, 0.0, 10.0],
            ..Default::default()
        };
        let mut state = state(0);

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].rot_start, 91);
        assert_eq!(state.slots[0].rise_angle, 30);
        assert_eq!(state.slots[0].alpha_b, 0);
    }

    #[test]
    fn fades_in_and_then_out() {
        let angles = Angles::new();
        let mut state = state(1);

        on_process(&mut state, &angles, &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 25);

        state.slots[0].process = 10;
        state.slots[0].alpha_b = 100;
        on_process(&mut state, &angles, &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 95);
    }

    #[test]
    fn trail_slots_copy_the_lead_every_third_frame() {
        let angles = Angles::new();
        let mut state = state(2);
        state.slots[0].alpha_b = 100;
        state.slots[0].vec_b_now = [5.0, 0.0, 7.0];
        state.slots[0].rise_angle = 40;
        state.slots[0].height[1] = 0.0;

        on_process(&mut state, &angles, &MasterState::default());

        assert_eq!(state.slots[1].vec_b_now, [5.0, 0.0, 7.0]);
        assert_eq!(state.slots[1].alpha_b, 100);
        assert_eq!(state.slots[1].rise_angle, 70);
    }
}
