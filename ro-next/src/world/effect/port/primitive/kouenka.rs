//! `PP_KOUENKA` (124) - `CEffectPrim::PrimKouenka`,
//! `EffectPrimitiveMotion.cpp:1713-1770`; dispatch
//! `EffectPrimitiveMotion.cpp:9738-9739`.
//!
//! `CRagEffect::SakuraFireArrow` (`EF_KOUENKA`) launches one of these per
//! frame for its first twenty frames: a `sakura01` sprite that floats up,
//! wanders by a pair of phase angles (`height[0]`/`height[1]` and
//! `height[2]`/`height[3]`) and fades out through `ProcessAlpha`.
//!
//! Quirk kept: the dispatch calls `PrimKouenka()` as a plain statement, so the
//! `AnimMotion` result is discarded and a finished sprite animation never
//! retires the primitive - it lives until `stateCnt > duration`.
//!
//! The renderer is the shared [`RenderSegmentSprites2`]
//! (`EffectPrimitiveRendering.cpp:276`) sprite pass, like the other
//! particle-sprite primitives.

use super::particle3d::anim_motion;
use super::{EffectPrimitiveState, MasterState};
use crate::world::effect::port::angles::Angles;
use crate::world::effect::port::runtime::EffectRng;

/// One frame of `PrimKouenka` (`EffectPrimitiveMotion.cpp:1713-1770`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut EffectRng) -> bool {
    // `FollowMaster(*this)` (`EffectPrimitiveMotion.cpp:115-119`).
    state.position = master.position;

    for index in 0..1usize {
        if state.slots[index].life != 1 {
            continue;
        }

        for i in [0usize, 2] {
            let rising = state.slots[index].height[i + 1] > state.slots[index].height[i];

            if rising {
                let slot = &mut state.slots[index];

                slot.height[i] += 3.0;

                if slot.height[i] > 359.0 {
                    slot.height[i] = 359.0;
                }

                if slot.height[i] > slot.height[i + 1] {
                    slot.height[i + 1] = rng.random(slot.height[i] as i32) as f32;
                }
            } else {
                let slot = &mut state.slots[index];

                slot.height[i] -= 2.0;

                if slot.height[i] < 0.0 {
                    slot.height[i] = 0.0;
                }

                if slot.height[i] <= slot.height[i + 1] {
                    slot.height[i + 1] = (359 - rng.random(slot.height[i] as i32)) as f32;
                }
            }
        }

        let max_height = state.slots[index].max_height;
        let phase_x = state.slots[index].height[0] as i32 as usize;
        let phase_z = state.slots[index].height[2] as i32 as usize;

        // Quirk kept: the clamp compares against 3, not 0.
        state.alpha -= 3.0;

        if state.alpha < 3.0 {
            state.alpha = 0.0;
        }

        {
            let slot = &mut state.slots[index];

            slot.vec_b_pre[1] -= max_height;
            slot.vec_b_pre[0] += 0.15 * angles.raw_sine(phase_x);
            slot.vec_b_pre[2] += 0.15 * angles.raw_sine(phase_z);
        }

        state.delta_pos = state.slots[index].vec_b_pre;
    }

    // `this->render.position = this->state.deltaPos;`
    state.position = state.delta_pos;

    // `if ((this->state.pattern & 0x20) != 0) this->state.roll = this->render.angle
    // - camera->camera.yaw;`
    if (state.pattern & 0x20u32) != 0 {
        state.roll = state.render_angle - master.camera_yaw;
    }

    state.process_alpha(master);
    state.shift_segment();

    // Quirk kept: the official case discards `AnimMotion`'s return value.
    let _ = anim_motion(state);

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    /// `ProcessAlpha`'s `LimitAlpha` divides by the master's remaining
    /// lifetime when it has to fade the primitive out; a zero-length master
    /// would divide by zero, so the harness carries a real 300-frame one.
    fn master() -> MasterState {
        MasterState {
            state_cnt: 0,
            duration: 300,
            ..Default::default()
        }
    }

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Kouenka,
            duration: 300,
            alpha: 250.0,
            num_segment: 1,
            ..Default::default()
        };

        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            max_height: 0.3,
            height: {
                let mut height = [0.0f32; 21];
                height[0] = 100.0;
                height[1] = 200.0;
                height[2] = 50.0;
                height[3] = 60.0;
                height
            },
            ..Default::default()
        };

        state
    }

    #[test]
    fn the_sprite_floats_up_and_fades_three_per_frame() {
        let angles = Angles::new();
        let mut rng = EffectRng::new(7);
        let mut state = state();

        assert!(on_process(&mut state, &angles, &master(), &mut rng));

        assert!((state.slots[0].vec_b_pre[1] + 0.3).abs() < 1e-6);
        assert_eq!(state.alpha, 247.0);
        // `ShiftSegment` copied the primitive's position into record 0.
        assert_eq!(state.records[0].position, state.position);
        assert_eq!(state.records[0].color >> 24, state.alpha as u32);
    }

    #[test]
    fn alpha_bottoms_out_at_zero_below_three() {
        let angles = Angles::new();
        let mut rng = EffectRng::new(7);
        let mut state = state();
        state.alpha = 4.0;

        on_process(&mut state, &angles, &master(), &mut rng);

        assert_eq!(state.alpha, 0.0);
    }

    #[test]
    fn the_phase_angles_climb_three_per_frame_while_the_pair_rises() {
        let angles = Angles::new();
        let mut rng = EffectRng::new(7);
        let mut state = state();

        on_process(&mut state, &angles, &master(), &mut rng);

        // `height[i + 1] > height[i]` climbs `height[i]` by 3: 200 > 100 and
        // 60 > 50.
        assert!((state.slots[0].height[0] - 103.0).abs() < 1e-6);
        assert!((state.slots[0].height[2] - 53.0).abs() < 1e-6);
    }

    #[test]
    fn a_falling_pair_drops_two_per_frame_and_repicks_the_next_phase() {
        let angles = Angles::new();
        let mut rng = EffectRng::new(7);
        let mut state = state();
        state.slots[0].height[0] = 300.0;
        state.slots[0].height[1] = 100.0;

        on_process(&mut state, &angles, &master(), &mut rng);

        // 100 is not above 300, so `height[0]` drops by 2 and `height[1]` is
        // re-picked as `359 - Random(298)`, i.e. in (61, 359].
        assert!((state.slots[0].height[0] - 298.0).abs() < 1e-6);
        assert!(state.slots[0].height[1] > 61.0 && state.slots[0].height[1] <= 359.0);
    }

    #[test]
    fn the_finished_animation_is_ignored() {
        let angles = Angles::new();
        let mut rng = EffectRng::new(7);
        let mut state = state();
        state.repeat_anim = 0;
        state.action_frame_count = 1;
        state.cur_motion = 0;
        state.anim_speed = 1;

        // `AnimMotion` would return false (`curMotion == FrameCount`), but the
        // dispatch discards it, so `PrimKouenka` still reports alive.
        assert!(on_process(&mut state, &angles, &master(), &mut rng));
    }
}
