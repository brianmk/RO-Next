//! prim 126 (no `EffectPrimId.h` symbol) - the inline `case 126:`
//! (`EffectPrimitiveMotion.cpp:9457-9505`).
//!
//! The four cross shards of `CRagEffect::TexturedSlashCross` (`EF_STIN5`).
//! Only slot 0 is stepped: `riseAngle += 30`, frame 1 aims at the target, the
//! next ten frames grow `alphaB` to 60 and `height[1]` by 2%, then it fades by
//! 3 per frame and 4% growth until alpha reaches zero and the slot dies.
//! Slots 1..3 receive the trail copy every third frame (the body arms them
//! with `alphaB = 0`, so they stay invisible).
//!
//! Renderer: [`render_rotated_ring`](super::super::render_slots::render_rotated_ring)
//! (`RenderRotatedRing`, `EffectPrimitiveRendering.cpp:2608-2635`, dispatch
//! `:554`).

use super::{Angles, EffectPrimitiveState, MasterState, get_angle};

/// One frame of the inline `case 126` (`EffectPrimitiveMotion.cpp:9457-9505`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
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

        if slot.rise_angle > 359 {
            slot.rise_angle -= 360;
        }

        if slot.process == 1 {
            slot.rot_start = get_angle(
                master.master_delta_pos[0] - state.position[0],
                master.master_delta_pos[2] - state.position[2],
            );
        } else if slot.process < 11 {
            slot.alpha_b += 6;

            if slot.alpha_b > 60 {
                slot.alpha_b = 60;
            }

            slot.height[1] *= 1.02;
        } else {
            slot.alpha_b -= 3;
            slot.height[1] *= 1.04;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
                slot.life = 0;
            }
        }

        slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start as usize);
        slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start as usize);
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
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state(process: i32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::SlashCross,
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
        state.slots[0].height[1] = 2.0;
        state
    }

    #[test]
    fn first_frame_aims_and_turns_thirty_degrees() {
        let angles = Angles::new();
        let master = MasterState {
            master_delta_pos: [0.0, 0.0, 10.0],
            ..Default::default()
        };
        let mut state = state(0);

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].rot_start, 91);
        assert_eq!(state.slots[0].rise_angle, 30);
        assert_eq!(state.slots[0].height[1], 2.0);
    }

    #[test]
    fn grows_for_ten_frames_then_fades() {
        let angles = Angles::new();
        let mut state = state(0);
        let master = MasterState::default();

        on_process(&mut state, &angles, &master);
        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].alpha_b, 6);
        assert!((state.slots[0].height[1] - 2.0 * 1.02).abs() < 1e-6);

        state.slots[0].process = 10;
        state.slots[0].alpha_b = 2;
        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 0);
    }
}
