//! prim 92 - the bare `case 92:` in `CEffectPrim::OnProcess`,
//! `EffectPrimitiveMotion.cpp:9367-9399`; dispatch `:9367`.
//!
//! The eight purple slashes of `EF_SOULBREAKER2` (409), spawned once per
//! 45-degree step by `CRagEffect::PurpleSlash` (`RagEffect.cpp:3382-3411`).
//! `EffectPrimId.h` has no symbol for the id, so the port names the variant
//! after its only spawner.
//!
//! Four shards start with a negative `process` (`index - 28 - offset`), so they
//! appear staggered; each shard travels along its own `rotStart` by
//! `height[1]` units per frame, accelerates its step by 2% while the alpha
//! rises by 10 and by 4% while the alpha drains by 5. A shard whose alpha falls
//! below zero dies.
//!
//! The motion is the same as prim 86's minus the first-frame `Get_Angle` turn.
//! The renderer is the shared `RenderFloorSquare`
//! (`EffectPrimitiveRendering.cpp:2637`), dispatched by the `case 86`/`case 92`
//! arm at `EffectPrimitiveRendering.cpp:545-547` - see
//! [`super::super::render_slots::render_floor_square`].

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of the official `case 92:`
/// (`EffectPrimitiveMotion.cpp:9367-9399`).
///
/// The motion is `void`, so the primitive retires through the shared
/// `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    // The original does not follow the master here: `PurpleSlash` writes the
    // primitive's `render.position` once at spawn.
    let _ = master;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        // `const int process = slot.process++;`
        let process = slot.process;
        slot.process += 1;

        // `if (slot.process <= 0) continue;` - the gate reads the *post*
        // increment value.
        if slot.process <= 0 {
            continue;
        }

        if process != 0 {
            if process < 10 {
                slot.alpha_b += 10;
                slot.height[1] *= 1.02;
            } else {
                slot.alpha_b -= 5;
                slot.height[1] *= 1.04;

                if slot.alpha_b < 0 {
                    slot.alpha_b = 0;
                    slot.life = 0;
                }
            }
        }

        slot.vec_b_now[0] += slot.height[1] * angles.raw_cosine(slot.rot_start as usize);
        slot.vec_b_now[2] += slot.height[1] * angles.raw_sine(slot.rot_start as usize);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn slashes() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::PurpleSlash,
            duration: 300,
            size: 0.0,
            ..Default::default()
        };
        state.position = [0.0, 0.0, 0.0];

        for slot in &mut state.slots {
            slot.life = 1;
            slot.process = 0;
            slot.distance = 20.0;
            slot.alpha_b = 0;
            slot.height[1] = 0.8;
            slot.rot_start = 0;
        }

        state
    }

    #[test]
    fn purple_slash_is_primitive_92() {
        assert_eq!(PrimitiveId::PurpleSlash.id(), 92);
    }

    #[test]
    fn the_first_frame_moves_the_shard_without_ramping() {
        let mut state = slashes();

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert!((state.slots[0].vec_b_now[0] - 0.8).abs() < 1e-6);
        assert_eq!(state.slots[0].vec_b_now[2], 0.0);
    }

    #[test]
    fn frames_one_to_nine_ramp_and_accelerate() {
        let mut state = slashes();
        state.slots[0].process = 1;

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].process, 2);
        assert_eq!(state.slots[0].alpha_b, 10);
        assert!((state.slots[0].height[1] - 0.8 * 1.02).abs() < 1e-6);
    }

    #[test]
    fn from_frame_ten_the_alpha_drains_and_the_shard_dies_below_zero() {
        let mut state = slashes();
        state.slots[0].process = 10;
        state.slots[0].alpha_b = 10;

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].process, 11);
        assert_eq!(state.slots[0].alpha_b, 5);
        assert!((state.slots[0].height[1] - 0.8 * 1.04).abs() < 1e-6);

        state.slots[0].alpha_b = 3;
        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 0);
    }

    #[test]
    fn a_negative_process_only_advances_until_it_reaches_zero() {
        let mut state = slashes();
        state.slots[0].process = -2;

        on_process(&mut state, Angles::shared(), &MasterState::default());
        assert_eq!(state.slots[0].process, -1);
        assert_eq!(state.slots[0].vec_b_now, [0.0; 3]);

        on_process(&mut state, Angles::shared(), &MasterState::default());
        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].vec_b_now, [0.0; 3]);

        on_process(&mut state, Angles::shared(), &MasterState::default());
        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].vec_b_now[0], 0.8);
    }

    #[test]
    fn the_shard_travels_along_its_own_rot_start() {
        let mut state = slashes();
        state.slots[0].rot_start = 90;

        on_process(&mut state, Angles::shared(), &MasterState::default());

        assert!(state.slots[0].vec_b_now[0].abs() < 1e-6);
        assert!((state.slots[0].vec_b_now[2] - 0.8).abs() < 1e-6);
    }
}
