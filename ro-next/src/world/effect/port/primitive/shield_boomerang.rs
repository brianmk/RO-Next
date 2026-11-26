//! prim 42 (`PP_SHIELDBOOMERANG`) - `CEffectPrim::PrimShieldBoomerang`,
//! `EffectPrimitiveMotion.cpp:1563-1709`; dispatch at
//! `EffectPrimitiveMotion.cpp:9802-9804`.
//!
//! The shield throw: one lead slot flies from the caster to `deltaPos` and
//! back, with three trailing copies. `flag1[2]` picks the two flight modes the
//! bodies use:
//!
//! * `0`: `TexturedShieldThrow` (`CRagEffect::TexturedShieldThrow`,
//!   `RagEffect.cpp:5688-5729`) - trail copies every third frame, rise until
//!   `maxHeight < height[1] * process`, then turn towards the caster (5 degrees
//!   per frame), then sink and fade by 12 alpha per frame.
//! * `1`: `CRagEffect::TANJI` (`RagEffect.cpp:14501-14542`) - trail copies
//!   every frame, alpha ramps 25/frame for ten frames, and the first time the
//!   slot passes its reach it flips to the return leg (`flag1[1] = 2`), triples
//!   `height[1]` and jitters `rotStart` by a random 0..44 degrees.
//!
//! The renderer is the shared `RenderFieldPanel`
//! (`EffectPrimitiveRendering.cpp:3312-3505`); `PP_SHIELDBOOMERANG` is in the
//! renderer's case list at `EffectPrimitiveRendering.cpp:573-592`.

use super::super::angles::Angles;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState, get_angle};

/// One frame of prim 42. `PrimShieldBoomerang` returns `void`, so the
/// primitive only retires through the shared `stateCnt > duration` tail.
///
/// `CEffectPrim::PrimShieldBoomerang` - `EffectPrimitiveMotion.cpp:1563-1709`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut EffectRng) {
    face_camera(&mut state.slots, master);

    // `FollowMaster(*this);` - `EffectPrimitiveMotion.cpp:1570`.
    state.position = master.position;

    // `for (int ec = 0; ec < 1; ++ec)`: only slot zero is the lead slot.
    if state.slots[0].life != 1 {
        return;
    }

    let mut slot = state.slots[0];
    slot.process += 1;

    // `if (this->state.slots[ec].process > 0) { ... }`
    if slot.process > 0 {
        // `ra` is 1 for the TANJI mode and 3 otherwise.
        let ra = if slot.flag1[2] == 1 { 1 } else { 3 };

        // `if (flag1[2] == 1) { if (process <= 10) alphaB += 25; if (alphaB >=
        // 255) alphaB = 255; }`
        if slot.flag1[2] == 1 {
            if slot.process <= 10 {
                slot.alpha_b += 25;
            }

            if slot.alpha_b >= 255 {
                slot.alpha_b = 255;
            }
        }

        if slot.process % ra == 0 {
            if slot.process > 3 {
                let source = state.slots[2];
                copy_trail(&mut state.slots[3], &source, 25);
            }

            if slot.process > 2 {
                let source = state.slots[1];
                copy_trail(&mut state.slots[2], &source, 25);
            }

            if slot.process > 1 {
                let source = state.slots[0];
                copy_trail(&mut state.slots[1], &source, 150);
            }
        }

        // `slots[ec].fullDisplayAngle -= 15;` with one 360 fixup.
        slot.full_display_angle -= 15;

        if slot.full_display_angle < 0 {
            slot.full_display_angle += 360;
        }

        // `if (process == 1) Get_Angle(deltaPos.x - vecBNow.x, deltaPos.z -
        // vecBNow.z, &rotStart);`
        if slot.process == 1 {
            let rx = state.delta_pos[0] - slot.vec_b_now[0];
            let rz = state.delta_pos[2] - slot.vec_b_now[2];
            slot.rot_start = get_angle(rx, rz);
        }

        if slot.flag1[1] == 0 {
            // The reach test doubles as the outbound/return switch.
            if slot.max_height < slot.height[1] * slot.process as f32 {
                if slot.flag1[2] == 1 {
                    slot.flag1[1] = 2;
                    slot.height[1] *= 3.0;

                    // `if (ClampedRandom(2) == 0) rotStart +=
                    // ClampedRandom(45); else rotStart -= ClampedRandom(45);`
                    // The two draws happen in that order in both branches.
                    if rng.clamped_random(2) == 0 {
                        slot.rot_start += rng.clamped_random(45) as i16;

                        if slot.rot_start >= 360 {
                            slot.rot_start -= 360;
                        }
                    } else {
                        slot.rot_start -= rng.clamped_random(45) as i16;

                        if slot.rot_start < 0 {
                            slot.rot_start += 360;
                        }
                    }
                } else {
                    slot.flag1[1] = 1;
                }
            }

            // Quirk kept: when the transition above just set `flag1[1]`, the
            // vertical step still runs because the branch was already chosen.
            slot.vec_b_now[1] += slot.height[2];
        } else if slot.flag1[1] == 1 {
            let rx = state.position[0] - slot.vec_b_now[0];
            let rz = state.position[2] - slot.vec_b_now[2];
            let angle = get_angle(rx, rz);

            if slot.rot_start < angle - 4 || slot.rot_start > angle + 4 {
                slot.rot_start += 5;

                if slot.rot_start >= 360 {
                    slot.rot_start -= 360;
                }
            } else {
                slot.flag1[1] = 2;
            }
        } else {
            slot.alpha_b -= 12;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }

            // `vecBNow.y -= height[2] * 1.0f;`
            slot.vec_b_now[1] -= slot.height[2];
        }

        // The lead slot advances along `rotStart` in every branch.
        slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start as usize);
        slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start as usize);
    }

    state.slots[0] = slot;
}

/// The six-field trail copy shared by the shield primitives
/// (`EffectPrimitiveMotion.cpp:1595-1639`).
///
/// Quirk kept: `fullDisplayAngle` is assigned twice, exactly as the
/// decompilation emits it, and the alpha drop clamps after the subtraction.
fn copy_trail(destination: &mut EffectPrimitiveSlot, source: &EffectPrimitiveSlot, alpha_drop: i16) {
    destination.full_display_angle = source.full_display_angle;
    destination.distance = source.distance;
    destination.height[8] = source.height[8];
    destination.alpha_b = source.alpha_b - alpha_drop;

    if destination.alpha_b < 0 {
        destination.alpha_b = 0;
    }

    destination.vec_b_now = source.vec_b_now;
    destination.full_display_angle = source.full_display_angle;
    destination.flag1[0] = source.flag1[0];
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
        EffectPrimitiveState {
            count: PrimitiveId::ShieldBoomerang,
            duration: 499,
            ..Default::default()
        }
    }

    fn tick(state: &mut EffectPrimitiveState, rng: &mut EffectRng) {
        on_process(state, Angles::shared(), &MasterState::default(), rng);
    }

    #[test]
    fn primitive_id_is_42() {
        assert_eq!(PrimitiveId::ShieldBoomerang.id(), 42);
    }

    #[test]
    fn face_camera_writes_the_camera_angles_and_follows_the_master() {
        let master = MasterState {
            camera_yaw: 30.0,
            camera_pitch: 20.0,
            position: [4.0, 5.0, 6.0],
            ..Default::default()
        };
        let mut state = state();

        on_process(&mut state, Angles::shared(), &master, &mut EffectRng::new(1));

        assert_eq!(state.slots[0].alpha_t, 330);
        assert_eq!(state.slots[1].alpha_t, 70);
        assert_eq!(state.position, [4.0, 5.0, 6.0]);
    }

    #[test]
    fn tanji_mode_ramps_alpha_by_25_per_frame_up_to_255() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[2] = 1;
        state.slots[0].max_height = 1000.0;
        state.slots[0].height[1] = 2.0;
        let mut rng = EffectRng::new(1);

        for frame in 1..=10 {
            tick(&mut state, &mut rng);

            assert_eq!(state.slots[0].alpha_b, frame as i16 * 25);
        }

        // Frame 10 would push 250 + 25 over the top; the clamp pulls it back.
        state.slots[0].process = 9;
        state.slots[0].alpha_b = 250;
        tick(&mut state, &mut rng);
        assert_eq!(state.slots[0].alpha_b, 255);
    }

    #[test]
    fn tanji_mode_copies_the_trail_every_frame() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[2] = 1;
        state.slots[0].max_height = 1000.0;
        state.slots[0].height[1] = 2.0;
        state.slots[0].alpha_b = 200;
        state.slots[0].full_display_angle = 90;
        let mut rng = EffectRng::new(1);

        tick(&mut state, &mut rng);

        // ra == 1, process == 1 -> the `process > 1` copy does not run yet.
        assert_eq!(state.slots[1].alpha_b, 0);

        tick(&mut state, &mut rng);

        // Process 2: the copy runs before this frame's +25 alpha and -15
        // spin, so slot 1 gets alpha 200 + 25 = 225 and angle 90 - 15 = 75.
        assert_eq!(state.slots[1].alpha_b, 75);
        assert_eq!(state.slots[1].full_display_angle, 75);
        assert_eq!(state.slots[1].distance, 0.0);
    }

    #[test]
    fn tanji_reach_switches_to_the_return_leg_and_triples_the_step() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[2] = 1;
        state.slots[0].max_height = 5.0;
        state.slots[0].height[1] = 2.0;
        state.slots[0].height[2] = 1.5;
        state.slots[0].alpha_b = 255;
        let mut rng = EffectRng::new(7);

        // Process 1: 5 < 2 is false.
        tick(&mut state, &mut rng);
        assert_eq!(state.slots[0].flag1[1], 0);
        assert_eq!(state.slots[0].height[1], 2.0);
        assert_eq!(state.slots[0].vec_b_now[1], 1.5);

        // Process 2: 5 < 4 is false, still outbound.
        tick(&mut state, &mut rng);
        assert_eq!(state.slots[0].flag1[1], 0);
        assert_eq!(state.slots[0].height[1], 2.0);
        assert_eq!(state.slots[0].vec_b_now[1], 3.0);

        // Process 3: 5 < 6 -> return leg, height[1] tripled, and the same
        // frame still takes the +height[2] step of the outbound branch.
        tick(&mut state, &mut rng);
        assert_eq!(state.slots[0].flag1[1], 2);
        assert_eq!(state.slots[0].height[1], 6.0);
        assert_eq!(state.slots[0].vec_b_now[1], 4.5);

        // The return leg drops by 12 alpha and by height[2] per frame.
        tick(&mut state, &mut rng);
        assert_eq!(state.slots[0].alpha_b, 255 - 12);
        assert_eq!(state.slots[0].vec_b_now[1], 3.0);
    }

    #[test]
    fn shield_throw_mode_switches_to_the_homing_leg_at_its_reach() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].max_height = 5.0;
        state.slots[0].height[1] = 2.0;
        state.slots[0].height[2] = 1.0;
        let mut rng = EffectRng::new(1);

        // flag1[2] == 0: ra == 3, so process 1 and 2 do not copy.
        tick(&mut state, &mut rng);
        tick(&mut state, &mut rng);
        assert_eq!(state.slots[1].alpha_b, 0);

        // process 3: the trail refreshes and 5 < 6 flips to the homing leg.
        tick(&mut state, &mut rng);
        assert_eq!(state.slots[1].alpha_b, 0, "slot 0 alpha is 0, so is 0 - 150");
        assert_eq!(state.slots[0].flag1[1], 1);
        assert_eq!(state.slots[0].vec_b_now[1], 3.0);
    }

    #[test]
    fn homing_leg_turns_five_degrees_until_aligned_then_descends() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].flag1[1] = 1;
        // Start past frame 1 so the `process == 1` heading write does not
        // overwrite the rotStart this test sets.
        state.slots[0].process = 2;
        state.slots[0].rot_start = 90;
        state.slots[0].height[1] = 2.0;
        state.slots[0].height[2] = 2.0;
        state.slots[0].alpha_b = 255;
        // Target direction from vecBNow (10, 0, 0) to position (0, 0, 0) is
        // Get_Angle(-10, 0) = 180; 90 is far from 180, so the slot turns.
        state.slots[0].vec_b_now = [10.0, 0.0, 0.0];
        let mut rng = EffectRng::new(1);
        let angles = Angles::shared();

        on_process(&mut state, angles, &MasterState::default(), &mut rng);

        // The homing branch does not apply the vertical step.
        assert_eq!(state.slots[0].vec_b_now[1], 0.0);
        assert_eq!(state.slots[0].flag1[1], 1);
        // rotStart moved +5 and then the horizontal step was taken.
        let turned = 95i16;
        assert_eq!(state.slots[0].vec_b_now[0], 10.0 + 2.0 * angles.raw_cosine(turned as usize));

        // Aim the slot almost at the caster: Get_Angle(0, 10) == 91, and
        // rotStart 90 is within the four-degree window, so the next frame
        // switches to the sink branch (alpha -12, y -height[2]).
        state.slots[0].process = 5;
        state.slots[0].rot_start = 90;
        state.slots[0].vec_b_now = [0.0, 0.0, -10.0];
        on_process(&mut state, angles, &MasterState::default(), &mut rng);

        // The aligned frame only latches `flag1[1] = 2`; the sink branch runs
        // on the following frame.
        assert_eq!(state.slots[0].flag1[1], 2);
        assert_eq!(state.slots[0].alpha_b, 255);
        assert_eq!(state.slots[0].vec_b_now[1], 0.0);

        on_process(&mut state, angles, &MasterState::default(), &mut rng);

        assert_eq!(state.slots[0].alpha_b, 243);
        assert_eq!(state.slots[0].vec_b_now[1], -2.0);
    }

    #[test]
    fn process_one_heads_towards_the_effect_delta_pos() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].max_height = 1000.0;
        state.slots[0].height[1] = 2.0;
        state.delta_pos = [0.0, 0.0, 10.0];
        let mut rng = EffectRng::new(1);

        tick(&mut state, &mut rng);

        // Get_Angle(0, 10) == 91 in the client's table.
        assert_eq!(state.slots[0].rot_start, 91);
    }

    #[test]
    fn process_at_or_below_zero_does_not_move_the_slot() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].process = -1;
        state.slots[0].alpha_b = 255;
        state.slots[0].vec_b_now = [1.0, 2.0, 3.0];
        let mut rng = EffectRng::new(1);

        tick(&mut state, &mut rng);

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].alpha_b, 255);
        assert_eq!(state.slots[0].vec_b_now, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn a_dead_lead_slot_does_not_touch_the_trails() {
        let mut state = state();
        state.slots[1].alpha_b = 77;
        let mut rng = EffectRng::new(1);

        tick(&mut state, &mut rng);

        assert_eq!(state.slots[1].alpha_b, 77);
    }
}
