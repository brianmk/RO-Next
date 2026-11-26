//! prim 155 (`PrimWanderingFlames`) - `CEffectPrim::PrimWanderingFlames`,
//! `EffectPrimitiveMotion.cpp:7353-7403`; dispatch at
//! `EffectPrimitiveMotion.cpp:8799-8801`.
//!
//! Four slots drift around the master as twin glow squares. Each slot carries
//! two wander angles in `height[0..5]` (pre) and `height[5..10]` (now); the
//! angles step toward a per-pair target and are re-rolled with `rand()` when
//! they reach it. `height[12]/[13]` are the alpha fade track. The helper
//! functions are the file-local inlines at `EffectPrimitiveMotion.cpp:7301-7351`
//! (`StepTowardAngle`, `WrapAngle`, `MoveWanderingSlot`, `FaceCamera`).
//!
//! `CRagEffect::TexturedForkBolt` (`RagEffect.cpp:7268-7332`) is the caller
//! ported in `rag_batch_a.rs`.

use super::super::angles::Angles;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState};

/// One frame of `PrimWanderingFlames`; it always returns `true`.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut EffectRng) -> bool {
    face_camera(&mut state.slots, master);

    for index in 0..4 {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        for pair in 0..2 {
            let base = pair * 5;

            let mut angle = slot.height[base + 2];
            let target = slot.height[base + 1];

            if step_toward_angle(&mut angle, target, 2.0) {
                slot.height[base + 1] = slot.height[base] - 45.0 + rng.clamped_random(91) as f32;
            }

            slot.height[base + 2] = angle;

            let mut angle = slot.height[base + 4];
            let target = slot.height[base + 3];

            if step_toward_angle(&mut angle, target, 1.0) {
                slot.height[base + 3] = rng.clamped_random(11) as f32 + 175.0;
            }

            slot.height[base + 4] = angle;
        }

        for angle in 0..10 {
            wrap_angle(&mut slot.height[angle]);
        }

        for fade in 12..14 {
            if slot.process < 10 {
                slot.height[fade] += 25.0;

                if slot.height[fade] >= 255.0 {
                    slot.height[fade] = 255.0;
                }
            } else if slot.process > 20 {
                slot.height[fade] -= 3.0;

                if slot.height[fade] < 0.0 {
                    slot.life = 0;
                    slot.height[fade] = 0.0;
                }
            }
        }

        move_wandering_slot(angles, slot);
    }

    true
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

/// `StepTowardAngle` - `EffectPrimitiveMotion.cpp:7304-7322`. Returns whether
/// the target was reached (the original's `retarget()` condition).
fn step_toward_angle(angle: &mut f32, target: f32, step: f32) -> bool {
    let difference = *angle - target;

    if difference < 180.0 && difference >= 0.0 {
        *angle -= step;
        *angle <= target
    } else if difference < -180.0 {
        *angle -= step;
        *angle <= target - 360.0
    } else {
        *angle += step;
        *angle >= target
    }
}

/// `WrapAngle` - `EffectPrimitiveMotion.cpp:7325-7331`.
fn wrap_angle(angle: &mut f32) {
    if *angle < 0.0 {
        *angle += 360.0;
    }

    if *angle >= 360.0 {
        *angle -= 360.0;
    }
}

/// `MoveWanderingSlot` - `EffectPrimitiveMotion.cpp:7334-7343`.
fn move_wandering_slot(angles: &Angles, slot: &mut EffectPrimitiveSlot) {
    let speed = slot.max_height;
    let cos_pre = angles.raw_cosine(slot.height[2] as usize);
    let sin_pre = angles.raw_sine(slot.height[2] as usize);
    let sin_pre_y = angles.raw_sine(slot.height[4] as usize);
    let cos_now = angles.raw_cosine(slot.height[7] as usize);
    let sin_now = angles.raw_sine(slot.height[7] as usize);
    let sin_now_y = angles.raw_sine(slot.height[9] as usize);

    slot.vec_b_pre[0] += speed * cos_pre;
    slot.vec_b_pre[2] += speed * sin_pre;
    slot.vec_b_pre[1] -= speed * sin_pre_y;

    slot.vec_b_now[0] += speed * cos_now;
    slot.vec_b_now[2] += speed * sin_now;
    slot.vec_b_now[1] += speed * sin_now_y;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: super::super::PrimitiveId::WanderingFlames,
            duration: 10,
            fade_out_cnt: 10,
            ..Default::default()
        }
    }

    fn close(left: f32, right: f32) -> bool {
        (left - right).abs() < 1e-5
    }

    #[test]
    fn face_camera_writes_the_slot_scroll_angles() {
        let angles = Angles::new();
        let master = MasterState {
            camera_yaw: 30.0,
            camera_pitch: 20.0,
            ..Default::default()
        };
        let mut rng = EffectRng::new(1);
        let mut state = state();

        on_process(&mut state, &angles, &master, &mut rng);

        // FaceCamera has no slot-life gate.
        assert_eq!(state.slots[0].alpha_t, 330);
        assert_eq!(state.slots[1].alpha_t, 70);
    }

    #[test]
    fn face_camera_keeps_the_unwrapped_360_quirk_on_slot_zero() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();

        on_process(&mut state, &angles, &master, &mut rng);

        // `360 - 0` is assigned to an int16 without a >=360 fixup.
        assert_eq!(state.slots[0].alpha_t, 360);
        // Slot one is fixed up: 360 - 0 + 90 = 450 -> 90.
        assert_eq!(state.slots[1].alpha_t, 90);
    }

    #[test]
    fn first_slot_frame_steps_wraps_fades_and_moves() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.max_height = 1.0;
        slot.process = 0;
        slot.height[0] = 90.0;
        slot.height[1] = 0.0;
        slot.height[2] = 10.0;
        slot.height[3] = 180.0;
        slot.height[4] = 190.0;
        slot.height[5] = 90.0;
        slot.height[6] = 0.0;
        slot.height[7] = 10.0;
        slot.height[8] = 180.0;
        slot.height[9] = 190.0;

        on_process(&mut state, &angles, &master, &mut rng);

        let slot = &state.slots[0];
        assert_eq!(slot.process, 1);
        // Pair 0: 10 -> 8 (target 0, step 2); 190 -> 189 (target 180, step 1).
        assert_eq!(slot.height[2], 8.0);
        assert_eq!(slot.height[4], 189.0);
        // Pair 1: 10 -> 8 (target 0, step 2); 190 -> 189 (target 180, step 1).
        assert_eq!(slot.height[7], 8.0);
        assert_eq!(slot.height[9], 189.0);
        // Process < 10: both fade values gain 25, clamped at 255.
        assert_eq!(slot.height[12], 25.0);
        assert_eq!(slot.height[13], 25.0);
        // MoveWanderingSlot: pre = (cos 8, -sin 189, sin 8),
        // now = (cos 8, +sin 189, sin 8).
        assert!(close(slot.vec_b_pre[0], angles.raw_cosine(8)));
        assert!(close(slot.vec_b_pre[1], -angles.raw_sine(189)));
        assert!(close(slot.vec_b_pre[2], angles.raw_sine(8)));
        assert!(close(slot.vec_b_now[0], angles.raw_cosine(8)));
        assert!(close(slot.vec_b_now[1], angles.raw_sine(189)));
        assert!(close(slot.vec_b_now[2], angles.raw_sine(8)));
    }

    #[test]
    fn reached_targets_are_rerolled_in_pair_order() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(99);
        let mut state = state();

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.max_height = 0.0;
        slot.process = 0;
        slot.height[0] = 100.0;
        slot.height[1] = 100.0;
        slot.height[2] = 100.0;
        slot.height[3] = 180.0;
        slot.height[4] = 180.0;
        slot.height[5] = 200.0;
        slot.height[6] = 200.0;
        slot.height[7] = 200.0;
        slot.height[8] = 180.0;
        slot.height[9] = 180.0;

        // RNG order: pair 0 pre-angle, pair 0 height, pair 1 pre-angle,
        // pair 1 height.
        let mut expected = EffectRng::new(99);
        let pair0_pre = expected.clamped_random(91) as f32;
        let pair0_height = expected.clamped_random(11) as f32;
        let pair1_pre = expected.clamped_random(91) as f32;
        let pair1_height = expected.clamped_random(11) as f32;

        on_process(&mut state, &angles, &master, &mut rng);

        let slot = &state.slots[0];
        assert_eq!(slot.height[1], 100.0 - 45.0 + pair0_pre);
        assert_eq!(slot.height[3], 175.0 + pair0_height);
        assert_eq!(slot.height[6], 200.0 - 45.0 + pair1_pre);
        assert_eq!(slot.height[8], 175.0 + pair1_height);
        // Pre-angle steps first: 100 -> 98, 200 -> 198.
        assert_eq!(slot.height[2], 98.0);
        assert_eq!(slot.height[7], 198.0);
    }

    #[test]
    fn wrap_angle_adds_and_subtracts_360_once() {
        let mut angle = -1.0;
        wrap_angle(&mut angle);
        assert_eq!(angle, 359.0);

        let mut angle = 360.5;
        wrap_angle(&mut angle);
        assert_eq!(angle, 0.5);

        let mut angle = 359.5;
        wrap_angle(&mut angle);
        assert_eq!(angle, 359.5);
    }

    #[test]
    fn fade_in_clamps_at_255_and_mid_range_holds() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.max_height = 0.0;
        slot.process = 8;
        slot.height[12] = 40.0;
        slot.height[13] = 240.0;

        on_process(&mut state, &angles, &master, &mut rng);

        let slot = &state.slots[0];
        // process 8 -> 9, still < 10.
        assert_eq!(slot.height[12], 65.0);
        assert_eq!(slot.height[13], 255.0);
        assert_eq!(slot.life, 1);

        // process 9 -> 10: neither the in nor the out branch runs.
        state.slots[0].process = 9;
        on_process(&mut state, &angles, &master, &mut rng);
        assert_eq!(state.slots[0].height[12], 65.0);
        assert_eq!(state.slots[0].height[13], 255.0);
    }

    #[test]
    fn fade_out_kills_the_slot_and_zeroes_the_negative_height() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();

        let slot = &mut state.slots[0];
        slot.life = 1;
        slot.max_height = 0.0;
        slot.process = 20;
        slot.height[12] = 1.0;
        slot.height[13] = 25.0;

        on_process(&mut state, &angles, &master, &mut rng);

        let slot = &state.slots[0];
        // process 20 -> 21, > 20.
        assert_eq!(slot.height[12], 0.0);
        assert_eq!(slot.height[13], 22.0);
        assert_eq!(slot.life, 0);
    }

    #[test]
    fn dead_slots_are_not_advanced() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();

        state.slots[0].process = 7;
        state.slots[0].height[12] = 3.0;

        on_process(&mut state, &angles, &master, &mut rng);

        assert_eq!(state.slots[0].process, 7);
        assert_eq!(state.slots[0].height[12], 3.0);
    }

    #[test]
    fn primitive_lives_through_its_duration_frame() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut rng = EffectRng::new(1);
        let mut state = state();
        state.duration = 2;

        assert!(state.on_process(&angles, &master, &mut rng));
        assert!(state.on_process(&angles, &master, &mut rng));
        assert!(!state.on_process(&angles, &master, &mut rng));
    }

    #[test]
    fn primitive_id_is_155() {
        assert_eq!(super::super::PrimitiveId::WanderingFlames.id(), 155);
    }
}
