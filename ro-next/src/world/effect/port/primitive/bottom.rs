//! prim 47 (`PP_BOTTOM`) and prim 49 (`PP_BOTTOMOUT`) - the two motion bodies
//! behind the bottom-song family.
//!
//! * `CEffectPrim::PrimBottom` - `EffectPrimitiveMotion.cpp:855-917`, dispatch
//!   `EffectPrimitiveMotion.cpp:10027-10029` (`case PP_BOTTOM:`).
//! * `CEffectPrim::PrimBottomOut` - `EffectPrimitiveMotion.cpp:918-966`,
//!   dispatch `EffectPrimitiveMotion.cpp:10087-10089` (`case PP_BOTTOMOUT:`).
//!
//! Both move four field-panel slots rooted at the master and both are drawn by
//! the shared `RenderFieldPanel` (`EffectPrimitiveRendering.cpp:3312-3505`,
//! dispatch `:575-576`), which
//! [`super::super::render_slots::render_field_panel`] already ports.
//! `PrimBottom` wakes only slot 0 but drags slots 3..1 behind it every sixth
//! frame; `PrimBottomOut` ramps every armed slot's `alphaB` in and
//! out and resets its `process` so the panel falls to the ground again.
//!
//! `FaceCamera` is the inline at `EffectPrimitiveMotion.cpp:100-114`; like the
//! other slot modules this file keeps its own copy rather than sharing one, so
//! each motion body reads as the original does.

use super::super::angles::Angles;
use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState};

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

/// `CEffectPrim::PrimBottom` - `EffectPrimitiveMotion.cpp:855-917`.
pub fn prim_bottom(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    face_camera(&mut state.slots, master);

    state.position = master.position;

    // `for (int ec = 0; ec < 1; ++ec)` - the original only advances slot 0.
    let slot = &mut state.slots[0];

    if slot.life != 1 {
        return;
    }

    slot.process += 1;

    if slot.process % 6 == 0 {
        // `for (i = 3; i > 0; --i)`, i.e. 3, 2, 1: each live trailing slot
        // copies the one in front of it and fades by 50.
        for index in (1..4).rev() {
            if state.slots[index].life <= 0 {
                continue;
            }

            let previous = state.slots[index - 1];
            let slot = &mut state.slots[index];

            slot.full_display_angle = previous.full_display_angle;
            slot.distance = previous.distance + 0.5;
            slot.height[8] = previous.height[8];
            slot.alpha_b = previous.alpha_b - 50;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }

            // The original assigns `fullDisplayAngle` twice; the second copy
            // is what survives and is kept here.
            slot.vec_b_now = previous.vec_b_now;
            slot.full_display_angle = previous.full_display_angle;
            slot.flag1[0] = previous.flag1[0];
            slot.flag1[2] = 2;
            slot.height[7] = 0.0;
        }
    }

    let slot = &mut state.slots[0];

    slot.height[8] += 5.0;

    if slot.height[8] >= 360.0 {
        slot.height[8] -= 360.0;
    }

    slot.rot_start += 1;

    if slot.rot_start >= 360 {
        slot.rot_start -= 360;
    }

    slot.vec_b_now[0] = state.position[0];
    slot.vec_b_now[2] = state.position[2];
    slot.vec_b_pre[1] = state.position[1] - 6.0;
    slot.vec_b_now[1] = slot.vec_b_pre[1] + 3.0 * angles.raw_sine(slot.rot_start.max(0) as usize);

    if slot.flag1[2] == 7 {
        slot.full_display_angle += 10;

        if slot.full_display_angle >= 360 {
            slot.full_display_angle -= 360;
        }
    }
}

/// `CEffectPrim::PrimBottomOut` - `EffectPrimitiveMotion.cpp:918-966`.
pub fn prim_bottom_out(state: &mut EffectPrimitiveState, master: &MasterState) {
    face_camera(&mut state.slots, master);

    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process <= 10 {
            slot.alpha_b += 25;
        } else {
            slot.alpha_b -= 5;

            if slot.alpha_b <= 0 {
                slot.alpha_b = 0;
                slot.process = 0;

                if slot.flag1[1] == 0 {
                    slot.vec_b_now[1] = state.position[1];
                } else {
                    slot.vec_b_now[1] = state.position[1] - 5.0;
                }
            }

            slot.vec_b_now[1] -= 0.2;
        }

        slot.height[8] += 5.0;

        if slot.height[8] >= 360.0 {
            slot.height[8] -= 360.0;
        }

        if slot.flag1[1] == 1 {
            slot.full_display_angle += 5;

            if slot.full_display_angle >= 360 {
                slot.full_display_angle -= 360;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: PrimitiveId::Bottom,
            duration: 29999,
            ..Default::default()
        }
    }

    #[test]
    fn primitive_ids_are_47_and_49() {
        assert_eq!(PrimitiveId::Bottom.id(), 47);
        assert_eq!(PrimitiveId::BottomOut.id(), 49);
    }

    #[test]
    fn face_camera_writes_the_two_slot_angles() {
        let angles = Angles::new();
        let mut state = state();
        let master = MasterState {
            camera_yaw: 10.0,
            camera_pitch: 20.0,
            ..Default::default()
        };

        prim_bottom(&mut state, &angles, &master);

        assert_eq!(state.slots[0].alpha_t, 350);
        // 360 - 20 + 90 = 430 -> 70.
        assert_eq!(state.slots[1].alpha_t, 70);

        // The inline has no slot-life gate.
        let mut dead = state;
        for slot in &mut dead.slots {
            slot.life = 0;
            slot.alpha_t = 0;
        }
        prim_bottom(&mut dead, &angles, &master);
        assert_eq!(dead.slots[0].alpha_t, 350);
        assert_eq!(dead.slots[1].alpha_t, 70);
    }

    #[test]
    fn prim_bottom_advances_slot_zero() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].process = 4;
        state.slots[0].rot_start = 90;
        state.slots[0].distance = 3.0;
        state.slots[0].height[8] = 358.0;
        let master = MasterState {
            position: [7.0, 8.0, 9.0],
            ..Default::default()
        };

        prim_bottom(&mut state, &angles, &master);

        assert_eq!(state.position, [7.0, 8.0, 9.0]);
        assert_eq!(state.slots[0].process, 5);
        // 358 + 5 = 363 -> 3.
        assert_eq!(state.slots[0].height[8], 3.0);
        assert_eq!(state.slots[0].rot_start, 91);
        assert_eq!(state.slots[0].vec_b_now[0], 7.0);
        assert_eq!(state.slots[0].vec_b_now[2], 9.0);
        assert_eq!(state.slots[0].vec_b_pre[1], 2.0);
        assert_eq!(state.slots[0].vec_b_now[1], 2.0 + 3.0 * angles.raw_sine(91));
    }

    #[test]
    fn prim_bottom_wraps_rotation_and_skips_dead_slot_zero() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].rot_start = 359;

        prim_bottom(&mut state, &angles, &MasterState::default());

        assert_eq!(state.slots[0].rot_start, 0);

        let mut dead = state;
        dead.slots[0].life = 2;
        dead.slots[0].process = 10;
        prim_bottom(&mut dead, &angles, &MasterState::default());
        assert_eq!(dead.slots[0].process, 10);
    }

    #[test]
    fn prim_bottom_shifts_the_trail_every_sixth_frame() {
        let angles = Angles::new();
        let mut state = state();

        state.slots[0].life = 1;
        state.slots[0].distance = 3.0;
        state.slots[0].alpha_b = 200;
        state.slots[0].full_display_angle = 45;
        state.slots[0].height[8] = 10.0;
        state.slots[0].vec_b_now = [1.0, 2.0, 3.0];
        state.slots[0].flag1[0] = 0;

        for index in 1..4 {
            state.slots[index].life = 1;
            state.slots[index].distance = 0.0;
            state.slots[index].alpha_b = 0;
            state.slots[index].flag1[2] = 9;
        }

        let master = MasterState {
            position: [4.0, 5.0, 6.0],
            ..Default::default()
        };

        // Frames 1..5 only advance slot 0.
        for _ in 0..5 {
            prim_bottom(&mut state, &angles, &master);
        }

        assert_eq!(state.slots[1].distance, 0.0);
        assert_eq!(state.slots[1].alpha_b, 0);

        // Frame 6 shifts slots 1..3 from slot 0's state *before* slot 0 is
        // refreshed to the master position.
        let previous = state.slots[0].vec_b_now;
        prim_bottom(&mut state, &angles, &master);

        assert_eq!(state.slots[1].distance, 3.5);
        assert_eq!(state.slots[1].alpha_b, 150);
        assert_eq!(state.slots[1].full_display_angle, 45);
        assert_eq!(state.slots[1].flag1[2], 2);
        assert_eq!(state.slots[1].vec_b_now, previous);
        assert_eq!(state.slots[2].alpha_b, 0);
    }

    #[test]
    fn prim_bottom_clamps_the_trail_alpha_at_zero() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].alpha_b = 20;
        state.slots[1].life = 1;

        for _ in 0..6 {
            prim_bottom(&mut state, &angles, &MasterState::default());
        }

        assert_eq!(state.slots[1].alpha_b, 0);
    }

    #[test]
    fn prim_bottom_kind_seven_spins_the_panel() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].full_display_angle = 355;
        state.slots[0].flag1[2] = 7;

        prim_bottom(&mut state, &angles, &MasterState::default());

        // 355 + 10 = 365 -> 5.
        assert_eq!(state.slots[0].full_display_angle, 5);

        let mut other = state;
        other.slots[0].flag1[2] = 2;
        other.slots[0].full_display_angle = 355;
        prim_bottom(&mut other, &angles, &MasterState::default());
        assert_eq!(other.slots[0].full_display_angle, 355);
    }

    fn bottom_out() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: PrimitiveId::BottomOut,
            duration: 29999,
            slots: [
                EffectPrimitiveSlot {
                    life: 1,
                    process: 0,
                    alpha_b: 0,
                    height: {
                        let mut height = [0.0; 21];
                        height[8] = 355.0;
                        height
                    },
                    ..Default::default()
                },
                EffectPrimitiveSlot::default(),
                EffectPrimitiveSlot::default(),
                EffectPrimitiveSlot::default(),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn prim_bottom_out_ramps_alpha_in_for_ten_frames() {
        let mut state = bottom_out();

        for frame in 1..=10 {
            prim_bottom_out(&mut state, &MasterState::default());
            assert_eq!(state.slots[0].process, frame);
            assert_eq!(state.slots[0].alpha_b, frame as i16 * 25);
        }

        // height[8] 355 + 5 * 10 = 405 -> 45.
        assert_eq!(state.slots[0].height[8], 45.0);
    }

    #[test]
    fn prim_bottom_out_fades_and_resets_on_the_ground() {
        let mut state = bottom_out();
        state.slots[0].process = 10;
        state.slots[0].alpha_b = 5;
        state.slots[0].vec_b_now[1] = 0.0;
        let master = MasterState {
            position: [0.0, 20.0, 0.0],
            ..Default::default()
        };

        // process 11 -> alphaB 5 - 5 = 0 -> reset, y = 20 - 0.2.
        prim_bottom_out(&mut state, &master);

        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].vec_b_now[1], 19.8);

        // `flag1[1] == 1` resets five units lower.
        let mut lifted = bottom_out();
        lifted.slots[0].process = 10;
        lifted.slots[0].alpha_b = 0;
        lifted.slots[0].flag1[1] = 1;
        prim_bottom_out(&mut lifted, &master);
        assert_eq!(lifted.slots[0].vec_b_now[1], 14.8);
    }

    #[test]
    fn prim_bottom_out_kind_one_spins_and_wraps() {
        let mut state = bottom_out();
        state.slots[0].flag1[1] = 1;
        state.slots[0].full_display_angle = 358;

        prim_bottom_out(&mut state, &MasterState::default());

        // 358 + 5 = 363 -> 3.
        assert_eq!(state.slots[0].full_display_angle, 3);

        let mut kind_zero = state;
        kind_zero.slots[0].flag1[1] = 0;
        kind_zero.slots[0].full_display_angle = 358;
        prim_bottom_out(&mut kind_zero, &MasterState::default());
        assert_eq!(kind_zero.slots[0].full_display_angle, 358);
    }

    #[test]
    fn prim_bottom_out_leaves_dead_slots_alone() {
        let mut state = bottom_out();
        state.slots[0].life = 0;
        state.slots[1].life = 1;
        state.slots[1].alpha_b = 110;

        prim_bottom_out(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[1].process, 1);
        assert_eq!(state.slots[1].alpha_b, 135);
    }
}
