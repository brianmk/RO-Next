//! prim 45 `PP_TEIHIT2` - `CEffectPrim::PrimTeiHit2`,
//! `EffectPrimitiveMotion.cpp:1840-1898`; dispatch `case PP_TEIHIT2:`
//! `EffectPrimitiveMotion.cpp:10018-10019`.
//!
//! The flurry quad primitive of `EF_TEIHIT2` and `EF_BACKSTAP`. Each live slot
//! is face-camera oriented, rotates by `-5` degrees per frame and carries a
//! per-slot velocity: `height[2]` is the vertical step, `height[1]` the
//! horizontal step along `rotStart`. The first positive `process` frame picks
//! `rotStart` from the bearing to `state.deltaPos` plus a random `-30..+30`,
//! then the alpha ramps `+15` for ten frames and falls `-3` until the slot
//! dies:
//!
//! ```cpp
//! ++slot.process;
//! if (slot.process <= 0) continue;
//! slot.fullDisplayAngle -= 5;
//! if (slot.fullDisplayAngle < 0) slot.fullDisplayAngle += 360;
//! if (slot.process == 1) { Get_Angle(...); slot.rotStart += Random(60) - 30; ... }
//! else if (slot.process <= 10) slot.alphaB += 15;
//! else { slot.alphaB -= 3; if (slot.alphaB < 0) { slot.alphaB = 0; slot.life = 0; } }
//! slot.vecBNow.y += slot.height[2];
//! slot.vecBNow.x += slot.height[1] * g_cosTable[slot.rotStart];
//! slot.vecBNow.z += slot.height[1] * g_sinTable[slot.rotStart];
//! ```
//!
//! The renderer is the shared
//! [`render_field_panel`](super::super::render_slots::render_field_panel)
//! (`RenderFieldPanel`, `EffectPrimitiveRendering.cpp:3312-3505`; the
//! `PP_TEIHIT2` case is at `:574`).

use super::super::angles::Angles;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState, get_angle};

/// One frame of `PrimTeiHit2` (`EffectPrimitiveMotion.cpp:1840-1898`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut EffectRng) {
    face_camera(&mut state.slots, master);

    // `this->render.position = this->state.master->render.position;`
    state.position = master.position;

    // The process-1 branch reads `state.deltaPos` and `render.position`; both
    // are constant for the whole pass.
    let to_target = [state.delta_pos[0] - state.position[0], state.delta_pos[2] - state.position[2]];

    for ec in 0..4usize {
        let slot = &mut state.slots[ec];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process <= 0 {
            continue;
        }

        slot.full_display_angle -= 5;

        if slot.full_display_angle < 0 {
            slot.full_display_angle += 360;
        }

        if slot.process == 1 {
            slot.rot_start = get_angle(to_target[0], to_target[1]);
            slot.rot_start += (rng.clamped_random(60) - 30) as i16;

            if slot.rot_start >= 360 {
                slot.rot_start -= 360;
            }

            if slot.rot_start < 0 {
                slot.rot_start += 360;
            }
        } else if slot.process <= 10 {
            slot.alpha_b += 15;
        } else {
            slot.alpha_b -= 3;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
                slot.life = 0;
            }
        }

        slot.vec_b_now[1] += slot.height[2];
        slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start.max(0) as usize);
        slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start.max(0) as usize);
    }
}

/// `FaceCamera(EffectPrimitiveSlot*)` - `EffectPrimitiveMotion.cpp:100-114`.
fn face_camera(slots: &mut [EffectPrimitiveSlot; 4], master: &MasterState) {
    let yaw = master.camera_yaw as i16 as i32;
    slots[0].alpha_t = (360 - yaw) as i16;

    let pitch = master.camera_pitch as i16 as i32;
    let mut alpha_t = 360 - pitch + 90;

    if alpha_t >= 360 {
        alpha_t -= 360;
    }

    slots[1].alpha_t = alpha_t as i16;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::TeiHit2,
            duration: 300,
            ..Default::default()
        };

        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].distance = 0.5;
        state.slots[0].height[1] = 1.0;
        state.slots[0].height[2] = 0.5;
        state.slots[0].alpha_b = 0;
        state.slots[0].full_display_angle = 0;
        state.slots[0].rot_start = 0;

        state
    }

    #[test]
    fn primitive_id_is_45() {
        assert_eq!(PrimitiveId::TeiHit2.id(), 45);
    }

    #[test]
    fn face_camera_writes_the_two_scroll_angles() {
        let mut state = state();
        let master = MasterState {
            camera_yaw: 30.0,
            camera_pitch: 20.0,
            ..Default::default()
        };

        on_process(&mut state, Angles::shared(), &master, &mut EffectRng::new(1));

        assert_eq!(state.slots[0].alpha_t, 330);
        assert_eq!(state.slots[1].alpha_t, 360 - 20 + 90 - 360);
    }

    #[test]
    fn negative_process_slots_do_not_move() {
        let mut state = state();
        state.slots[0].process = -5;
        state.slots[0].height[1] = 2.0;

        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));

        assert_eq!(state.slots[0].process, -4);
        assert_eq!(state.slots[0].vec_b_now, [0.0; 3]);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].full_display_angle, 0, "the rotation runs only after the gate");
    }

    #[test]
    fn the_first_positive_frame_sets_the_bearing_and_moves_the_quad() {
        let mut state = state();
        state.slots[0].process = 0;
        state.position = [0.0, 0.0, 0.0];
        state.delta_pos = [10.0, 0.0, 0.0];

        // A separate RNG with the same seed predicts the turn the body will
        // draw; the body itself must receive an untouched generator.
        let mut predictor = EffectRng::new(7);
        let expected_turn = predictor.clamped_random(60) - 30;
        let mut rng = EffectRng::new(7);

        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut rng);

        assert_eq!(state.slots[0].process, 1);
        // `Get_Angle(10, 0)` is 0 in the client trig table, then the random
        // turn is applied and wrapped into [0, 360).
        let mut expected = get_angle(10.0, 0.0) as i32 + expected_turn;
        if expected >= 360 {
            expected -= 360;
        }
        if expected < 0 {
            expected += 360;
        }
        assert_eq!(state.slots[0].rot_start as i32, expected);
        assert_eq!(state.slots[0].full_display_angle, 355);
        assert_eq!(state.slots[0].vec_b_now, [
            state.slots[0].height[1] * Angles::shared().raw_cosine(expected as usize),
            state.slots[0].height[2],
            state.slots[0].height[1] * Angles::shared().raw_sine(expected as usize)
        ]);
    }

    #[test]
    fn alpha_ramps_up_for_ten_frames_then_falls_and_kills_the_slot() {
        let mut state = state();
        state.slots[0].process = 1;

        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));
        assert_eq!(state.slots[0].alpha_b, 15, "process 2 is still <= 10");

        for _ in 0..8 {
            on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));
        }
        assert_eq!(state.slots[0].process, 10);
        assert_eq!(state.slots[0].alpha_b, 135);

        // process 11 -> -3.
        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));
        assert_eq!(state.slots[0].alpha_b, 132);

        for _ in 0..44 {
            on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));
        }

        // 132 - 44 * 3 == 0: the slot is still alive on the zero frame.
        assert_eq!(state.slots[0].process, 55);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 1);

        // One more -3 crosses below zero, which is the death test.
        on_process(&mut state, Angles::shared(), &MasterState::default(), &mut EffectRng::new(1));
        assert_eq!(state.slots[0].process, 56);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 0);
    }
}
