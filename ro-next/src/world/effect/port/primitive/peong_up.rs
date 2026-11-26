//! prim 96 (`PP_PEONGUP`) - `CEffectPrim::PrimPeongUp`,
//! `EffectPrimitiveMotion.cpp:3744-3778`; dispatch at
//! `EffectPrimitiveMotion.cpp:8851-8852`.
//!
//! Four slots rise with the frames, fade in by 5 alpha (frames 1-29) and out
//! by 2 (frames 30+), drift their angle pairs and float upwards by
//! `maxHeight` per frame. `CRagEffect::WhiteLightTrail` (`RagEffect.cpp:13451-
//! 13516`, `rag/white_light_trail.rs`) is the body that spawns it; the renderer
//! is [`super::super::render_slots::render_peong_up`].
//!
//! The file-local `FaceCamera` inline (`EffectPrimitiveMotion.cpp:100-114`) and
//! `DriftSlotAngles` (`:3108-3140`) are ported next to the primitive, with
//! `SafeRandom` (`:3099-3106`) delegating to the effect RNG exactly like the
//! original's `ClampedRandom`.
//!
//! The original's `std::int16_t limit` in `WhiteLightTrail` is uninitialised
//! for a `kind` other than 0/1; that undefined value cannot be reproduced. The
//! port defines both branches the body actually uses (0 -> 35, 1 -> 0) and
//! keeps the rest of the body exact.

use super::super::angles::Angles;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState};

/// One frame of `PrimPeongUp`. The motion function always returns `true`, so
/// the primitive only retires through the shared `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut EffectRng) -> bool {
    face_camera(&mut state.slots, master);

    for index in 0..4 {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        // `if (++slot.process <= 0) continue;`
        slot.process += 1;

        if slot.process <= 0 {
            continue;
        }

        // `const int16_t alpha = (int16_t)(slot.alphaB + (process >= 30 ? -2 : 5));`
        let alpha = i32::from(slot.alpha_b) + if slot.process >= 30 { -2 } else { 5 };
        slot.alpha_b = alpha as i16;

        if alpha < 0 {
            slot.alpha_b = 0;
        }

        drift_slot_angles(slot, 2, 3.0, 2.0, rng);

        let height = slot.vec_b_pre[1];

        // `if (height < 0.0f)` - the original only adds the sideways drift while
        // the slot floats above the ground.
        if height < 0.0 {
            slot.vec_b_pre[0] += angles.raw_sine(slot.height[0].max(0.0) as usize) * 0.03;
            slot.vec_b_pre[2] += angles.raw_sine(slot.height[2].max(0.0) as usize) * 0.03;
        }

        slot.vec_b_pre[1] = height - slot.max_height;
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

/// `SafeRandom(int range)` - `EffectPrimitiveMotion.cpp:3099-3106`.
fn safe_random(rng: &mut EffectRng, range: i32) -> i32 {
    if range == 0 {
        return 0;
    }

    rng.clamped_random(range)
}

/// `DriftSlotAngles(EffectPrimitiveSlot&, int count, float rise, float fall)` -
/// `EffectPrimitiveMotion.cpp:3108-3140`.
///
/// Each pair (`height[axis]`, `height[axis + 1]`) is a current angle and its
/// target. `count * 2` axes are visited in steps of two, i.e. the pairs
/// `(height[0], height[1])`, `(height[2], height[3])`, ...
fn drift_slot_angles(slot: &mut EffectPrimitiveSlot, count: usize, rise: f32, fall: f32, rng: &mut EffectRng) {
    for axis in (0..count * 2).step_by(2) {
        if slot.height[axis + 1] > slot.height[axis] {
            slot.height[axis] += rise;

            if slot.height[axis] > 359.0 {
                slot.height[axis] = 359.0;
            }

            if slot.height[axis] > slot.height[axis + 1] {
                slot.height[axis + 1] = safe_random(rng, slot.height[axis] as i32) as f32;
            }
        } else {
            slot.height[axis] -= fall;

            if slot.height[axis] < 0.0 {
                slot.height[axis] = 0.0;
            }

            if slot.height[axis] <= slot.height[axis + 1] {
                slot.height[axis + 1] = (359 - safe_random(rng, slot.height[axis] as i32)) as f32;
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
            count: PrimitiveId::PeongUp,
            duration: 100,
            ..Default::default()
        }
    }

    #[test]
    fn primitive_id_is_96_and_named_after_the_original_enum() {
        assert_eq!(PrimitiveId::PeongUp.id(), 96);
    }

    #[test]
    fn face_camera_writes_the_camera_angles() {
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
        // `360 - 0` is assigned to an int16 without a >= 360 fixup.
        let mut default_state = EffectPrimitiveState {
            count: PrimitiveId::PeongUp,
            duration: 100,
            ..Default::default()
        };
        on_process(&mut default_state, &angles, &MasterState::default(), &mut rng);
        assert_eq!(default_state.slots[0].alpha_t, 360);
        assert_eq!(default_state.slots[1].alpha_t, 90);
    }

    #[test]
    fn non_positive_process_does_not_alpha_or_move() {
        let angles = Angles::new();
        let mut rng = EffectRng::new(1);
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].process = -1;
        state.slots[0].alpha_b = 4;
        state.slots[0].vec_b_pre = [1.0, -2.0, 3.0];

        on_process(&mut state, &angles, &MasterState::default(), &mut rng);

        // `++process` is 0, so the `<= 0` early-out keeps every value.
        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].alpha_b, 4);
        assert_eq!(state.slots[0].vec_b_pre, [1.0, -2.0, 3.0]);
    }

    #[test]
    fn alpha_rises_by_five_and_falls_by_two_from_frame_thirty() {
        let angles = Angles::new();
        let mut rng = EffectRng::new(1);
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].process = 0;

        for frame in 1..=29 {
            on_process(&mut state, &angles, &MasterState::default(), &mut rng);

            assert_eq!(state.slots[0].process, frame);
            assert_eq!(state.slots[0].alpha_b, (frame * 5) as i16);
        }

        // Frame 30 is the first fall frame: 145 - 2 = 143.
        on_process(&mut state, &angles, &MasterState::default(), &mut rng);

        assert_eq!(state.slots[0].process, 30);
        assert_eq!(state.slots[0].alpha_b, 143);
    }

    #[test]
    fn the_slot_floats_up_by_max_height_every_frame() {
        let angles = Angles::new();
        let mut rng = EffectRng::new(7);
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].max_height = 0.5;
        state.slots[0].vec_b_pre = [0.0, -1.0, 0.0];
        state.slots[0].height[0] = 0.0;
        state.slots[0].height[1] = 0.0;
        state.slots[0].height[2] = 0.0;
        state.slots[0].height[3] = 0.0;

        on_process(&mut state, &angles, &MasterState::default(), &mut rng);

        // The slot is above ground after the subtraction; only the y changed
        // by `-maxHeight` (the drift is applied before it).
        assert_eq!(state.slots[0].vec_b_pre[1], -1.5);
    }

    #[test]
    fn drift_rewrites_the_target_when_it_is_crossed() {
        // Pair (0, 1): current 10, target 0 -> the else branch subtracts 2 and,
        // because 8 <= 0 is false, keeps the target. Pair (2, 3): current 10,
        // target 20 -> the rise branch adds 3 (13), still <= 20, keeps target.
        let mut rng = EffectRng::new(3);
        let mut slot = EffectPrimitiveSlot::default();
        slot.height[0] = 10.0;
        slot.height[1] = 0.0;
        slot.height[2] = 10.0;
        slot.height[3] = 20.0;

        drift_slot_angles(&mut slot, 2, 3.0, 2.0, &mut rng);

        assert_eq!(slot.height[0], 8.0);
        assert_eq!(slot.height[1], 0.0);
        assert_eq!(slot.height[2], 13.0);
        assert_eq!(slot.height[3], 20.0);

        // Now the pairs cross and the targets are re-rolled with the RNG.
        // Pair (0, 1): 8 -> 6, still > 0. Pair (2, 3): 13 -> 16, not > 20.
        // Repeat until the crossing happens.
        for _ in 0..20 {
            drift_slot_angles(&mut slot, 2, 3.0, 2.0, &mut rng);
        }

        assert!(slot.height[1] >= 0.0);
        assert!(slot.height[3] <= 359.0);
    }
}
