//! prim 116 (`PP_SHIELDBOOMERANG3`) - `CEffectPrim::PrimShieldBoomerang3`,
//! `EffectPrimitiveMotion.cpp:1467-1559`; dispatch at
//! `EffectPrimitiveMotion.cpp:9805-9807`.
//!
//! One slot flies while the three trailing slots copy it every third frame,
//! each with its own alpha drop (`-25`, `-25`, `-150`). The lead slot spins by
//! `-15` degrees per frame (the twin `PrimThrowItem` spins by `+10`), ramps its
//! alpha up by 45 for the first five frames (clamped at 255) and drains 10 per
//! frame from frame 15 on. The horizontal step is
//! `height[1] * (cos, sin)(rotStart)`, i.e. the slot keeps travelling along
//! its own heading.
//!
//! The renderer is the shared `RenderFieldPanel`
//! (`EffectPrimitiveRendering.cpp:3312-3505`); `PP_SHIELDBOOMERANG3` is in the
//! renderer's case list at `EffectPrimitiveRendering.cpp:573-592`. The body
//! that spawns it is `CRagEffect::ShieldBoomerang3`
//! (`RagEffect.cpp:12078-12177`, `rag/shield_boomerang.rs`).

use super::super::angles::Angles;
use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState};

/// One frame of prim 116. `PrimShieldBoomerang3` returns `void`, so the
/// primitive only retires through the shared `stateCnt > duration` tail.
///
/// `CEffectPrim::PrimShieldBoomerang3` - `EffectPrimitiveMotion.cpp:1467-1559`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    face_camera(&mut state.slots, master);

    // `this->render.position = master->render.position;` (FollowMaster inlined
    // by the compiler; the source spells it out at
    // `EffectPrimitiveMotion.cpp:1472-1473`).
    state.position = master.position;

    // `for (int ec = 0; ec < 1; ++ec)`: only slot zero is the lead slot.
    if state.slots[0].life != 1 {
        return;
    }

    // Work on a copy of the lead slot. The trail copies below read
    // `slots[0..3]`, which are *not* the lead slot, so the copy is exact.
    let mut slot = state.slots[0];
    slot.process += 1;

    // `if (this->state.slots[ec].process > 0)`
    if slot.process > 0 {
        // `char ra = 3; if (process % ra == 0) { ... }`
        if slot.process % 3 == 0 {
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

        // `if (process <= 5) alphaB += 45; if (alphaB > 255) alphaB = 255;`
        if slot.process <= 5 {
            slot.alpha_b += 45;

            if slot.alpha_b > 255 {
                slot.alpha_b = 255;
            }
        }

        // `if (process >= 15) alphaB -= 10; if (alphaB < 0) alphaB = 0;`
        if slot.process >= 15 {
            slot.alpha_b -= 10;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }

        // The lead slot advances along `rotStart`; the original indexes
        // `g_cosTable` / `g_sinTable` directly.
        slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start as usize);
        slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start as usize);
    }

    state.slots[0] = slot;
}

/// The six-field trail copy of `PrimShieldBoomerang3`
/// (`EffectPrimitiveMotion.cpp:1484-1496`, `:1500-1512`, `:1516-1528`).
///
/// Quirk kept: `fullDisplayAngle` is assigned twice (the decompiler emits the
/// same write before and after `vecBNow`), and the alpha drop is applied
/// before the clamp.
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
///
/// Kept local, exactly like the other primitive ports; the slot-life gate does
/// not apply, so the two camera angles are written even when no slot is alive.
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
            count: PrimitiveId::ShieldBoomerang3,
            duration: 499,
            ..Default::default()
        }
    }

    #[test]
    fn primitive_id_is_116() {
        assert_eq!(PrimitiveId::ShieldBoomerang3.id(), 116);
    }

    #[test]
    fn face_camera_writes_the_camera_angles_even_without_a_live_slot() {
        let master = MasterState {
            camera_yaw: 30.0,
            camera_pitch: 20.0,
            ..Default::default()
        };
        let mut state = state();

        on_process(&mut state, Angles::shared(), &master);

        assert_eq!(state.slots[0].alpha_t, 330);
        assert_eq!(state.slots[1].alpha_t, 70);
    }

    #[test]
    fn follows_the_master_position_before_the_life_gate() {
        let master = MasterState {
            position: [7.0, 8.0, 9.0],
            ..Default::default()
        };
        let mut state = state();

        on_process(&mut state, Angles::shared(), &master);

        assert_eq!(state.position, [7.0, 8.0, 9.0]);
    }

    #[test]
    fn first_five_frames_ramp_the_lead_alpha_by_45() {
        let mut state = state();
        state.slots[0].life = 1;

        for frame in 1..=5 {
            on_process(&mut state, Angles::shared(), &MasterState::default());

            assert_eq!(state.slots[0].process, frame);
            assert_eq!(state.slots[0].alpha_b, frame as i16 * 45);
        }

        assert_eq!(state.slots[0].alpha_b, 225, "45 * 5, below the 255 clamp");
    }

    #[test]
    fn the_alpha_ramp_clamps_at_255() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].alpha_b = 250;

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 255);
    }

    #[test]
    fn the_lead_alpha_holds_between_frames_six_and_fourteen_then_drains() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].process = 14;
        state.slots[0].alpha_b = 255;

        // Frame 15: the -10 branch runs.
        on_process(&mut state, Angles::shared(), &MasterState::default());
        assert_eq!(state.slots[0].process, 15);
        assert_eq!(state.slots[0].alpha_b, 245);

        state.slots[0].alpha_b = 5;
        on_process(&mut state, Angles::shared(), &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 0, "the drain clamps at zero");
    }

    #[test]
    fn full_display_angle_drops_fifteen_per_frame_and_wraps_at_zero() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].full_display_angle = 0;

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].full_display_angle, 345);
    }

    #[test]
    fn trail_copies_on_the_third_frame_with_the_official_alpha_drops() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].full_display_angle = 100;
        state.slots[0].distance = 5.0;
        state.slots[0].height[8] = 30.0;
        state.slots[0].alpha_b = 200;
        state.slots[0].vec_b_now = [1.0, 2.0, 3.0];
        state.slots[0].flag1[0] = 2;

        // Frames 1 and 2 do not copy.
        for _ in 0..2 {
            on_process(&mut state, Angles::shared(), &MasterState::default());
        }

        assert_eq!(state.slots[1].full_display_angle, 0);
        assert_eq!(state.slots[1].alpha_b, 0);

        // Frame 3: process == 3, so slot 2 (> 2) and slot 1 (> 1) are
        // refreshed, slot 3 is not (> 3 is false). The copy runs before slot
        // 0's -15 spin, so it sees the 70 left by frames 1 and 2.
        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[1].full_display_angle, 70);
        assert_eq!(state.slots[1].distance, 5.0);
        assert_eq!(state.slots[1].height[8], 30.0);
        // The alpha ramp also happens after the copy: 200 + 45 (frame 1)
        // + 45 (frame 2) clamps at 255, so the copy sees 255.
        assert_eq!(state.slots[1].alpha_b, 105, "255 - 150");
        assert_eq!(state.slots[1].vec_b_now, [1.0, 2.0, 3.0]);
        assert_eq!(state.slots[1].flag1[0], 2);

        // slot 2 copies slot 1's *pre-frame-3* values (all zero), not slot 0.
        assert_eq!(state.slots[2].alpha_b, 0);
        assert_eq!(state.slots[2].vec_b_now, [0.0, 0.0, 0.0]);
        // The lead slot itself has already decremented (70 - 15).
        assert_eq!(state.slots[0].full_display_angle, 55);
        assert_eq!(state.slots[3].alpha_b, 0);
    }

    #[test]
    fn trail_alpha_is_clamped_at_zero_for_the_first_link() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].alpha_b = 100;

        // Frame 3 copies slot 0 (alpha 145 after two +45 ramps? No: the copy
        // uses the value at copy time, which is 100 + 45 on frame 3).
        for _ in 0..3 {
            on_process(&mut state, Angles::shared(), &MasterState::default());
        }

        // The copy runs before frame 3's +45, so 100 + 45 * 2 - 150 = 40.
        assert_eq!(state.slots[1].alpha_b, 40);

        state.slots[0].alpha_b = 10;
        state.slots[0].process = 5;
        on_process(&mut state, Angles::shared(), &MasterState::default());

        // 10 + 45 = 55, minus 150 -> clamped to 0.
        assert_eq!(state.slots[1].alpha_b, 0);
    }

    #[test]
    fn the_lead_slot_moves_along_its_heading() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].height[1] = 2.5;
        state.slots[0].rot_start = 0;
        state.slots[0].vec_b_now = [10.0, 0.0, 10.0];

        on_process(&mut state, Angles::shared(), &MasterState::default());

        let angles = Angles::shared();
        assert_eq!(state.slots[0].vec_b_now[0], 10.0 + 2.5 * angles.raw_cosine(0));
        assert_eq!(state.slots[0].vec_b_now[2], 10.0 + 2.5 * angles.raw_sine(0));
    }

    #[test]
    fn slot_zero_only_advances_while_its_process_is_positive() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].process = -1;
        state.slots[0].alpha_b = 255;
        state.slots[0].full_display_angle = 90;

        // process -1 -> 0: the `process > 0` gate skips all motion.
        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].alpha_b, 255);
        assert_eq!(state.slots[0].full_display_angle, 90);
    }

    #[test]
    fn a_dead_lead_slot_does_not_touch_the_trails() {
        let mut state = state();
        state.slots[1].alpha_b = 77;
        state.slots[1].process = 5;

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[1].alpha_b, 77);
        assert_eq!(state.slots[1].process, 5);
    }
}
