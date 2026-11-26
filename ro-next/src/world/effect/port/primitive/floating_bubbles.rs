//! Prim 35 - `EffectPrimitiveMotion.cpp:6491-6533`
//! (`CEffectPrim::PrimFloatingBubbles`).
//!
//! The official enum has no symbol for this id (`EffectPrimId.h` jumps from
//! `PP_MAPPARTICLE` 27 to `PP_GI_3` 34). The port names it after its motion
//! body; the renderer is `RenderOrbitIcons`
//! (`EffectPrimitiveRendering.cpp:4764`, dispatch `:491-493`), called with
//! `(1.0f, 100, 100, 255)`.
//!
//! Four ascending bubbles per live slot. The four texture blocks
//! (`height[4k]..height[4k+3]` for `k = 0..3`) each hold
//! `(height, alpha, textureIndex, angle)`:
//!
//! * `height[bubble] += 1`; while it is below `10` the bubble rises
//!   (`height[bubble + 1] += 25`),
//! * once it reaches `10` it starts draining (`height[bubble + 1] -= 12`) and
//!   when the alpha falls below `12` the bubble resets: height and alpha zero,
//!   then `height[bubble + 3] = ClampedRandom(360)`, `height[bubble + 2] =
//!   ClampedRandom(5)` and the block's owner vector gets `y = -3 -
//!   ClampedRandom(8)`.
//!
//! RNG order, per block, is exactly **360, 5, 8** - the angle, the icon index
//! and the vertical offset, in that order (the offset draw is the last one).
//!
//! Quirks kept:
//!
//! * the reset order is `height[bubble+3]` (angle) *before* `height[bubble+2]`
//!   (texture index), matching the decompiled statements,
//! * `FaceCamera(this->state.slots)` is applied by the renderer (the simulation
//!   has no camera), like prim 82,
//! * `height[20]` receives the parent actor's name offset when the master has a
//!   parent. The runtime models a non-mount actor, whose
//!   `CGameActor::GetNameOffset()` is `0.0` (`GameActor.cpp:931`); the port
//!   keeps the field as an optional master input so a mounted caster can still
//!   supply it.
//!
//! Spawned by `CRagEffect::SuperSlashFan` (`RagEffect.cpp:16964`) -
//! `EF_GUMGANG` (203) and `EF_STEELBODY` (254).

use super::super::rng::EffectRng;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimFloatingBubbles`. Always returns `true`; the primitive
/// retires through the shared `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState, rng: &mut EffectRng) -> bool {
    // `FaceCamera(this->state.slots);` is applied by the renderer.

    // `this->render.position = master->render.position;`
    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        for bubble in (0..=12usize).step_by(4) {
            slot.height[bubble] += 1.0;

            if slot.height[bubble] < 10.0 {
                slot.height[bubble + 1] += 25.0;
            } else {
                slot.height[bubble + 1] -= 12.0;

                if slot.height[bubble + 1] < 12.0 {
                    slot.height[bubble] = 0.0;
                    slot.height[bubble + 1] = 0.0;
                    slot.height[bubble + 3] = rng.clamped_random(360) as f32;
                    slot.height[bubble + 2] = rng.clamped_random(5) as f32;

                    let y = -(3 + rng.clamped_random(8)) as f32;

                    match bubble {
                        0 => slot.vec_b_pre[1] = y,
                        4 => slot.vec_b_now[1] = y,
                        8 => slot.vec_t_pre[1] = y,
                        _ => slot.vec_t_now[1] = y,
                    }
                }
            }
        }

        if let Some(name_offset) = master.parent_name_offset {
            slot.height[20] = name_offset;
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn bubbles() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::FloatingBubbles,
            duration: 99999,
            ..Default::default()
        };
        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].max_height = 100.0;

        state
    }

    #[test]
    fn floating_bubbles_is_primitive_35() {
        assert_eq!(PrimitiveId::FloatingBubbles.id(), 35);
    }

    #[test]
    fn first_frame_advances_the_process_and_lifts_the_four_bubbles() {
        let mut state = bubbles();
        let mut rng = EffectRng::new(1);

        on_process(&mut state, &MasterState::default(), &mut rng);

        let slot = &state.slots[0];
        assert_eq!(slot.process, 1);
        assert_eq!(slot.height[0], 1.0);

        // Height 0 is below ten, so its alpha sample rises by 25.
        assert_eq!(slot.height[1], 25.0);
        assert_eq!(slot.height[4], 1.0);
        assert_eq!(slot.height[8], 1.0);
        assert_eq!(slot.height[12], 1.0);
    }

    #[test]
    fn a_drained_bubble_reset_draws_360_then_5_then_8() {
        let mut state = bubbles();
        state.slots[0].height[0] = 10.0;
        state.slots[0].height[4] = 10.0;
        state.slots[0].height[8] = 10.0;
        state.slots[0].height[12] = 10.0;

        // Every block drains for the first time; each reset consumes three
        // values in the order 360, 5, 8.
        let mut rng = EffectRng::new(7);
        let mut expected = rng.clone();

        // Block by block: angle (360), icon index (5), vertical offset (8).
        let draws: Vec<(i32, i32, i32)> = (0..4)
            .map(|_| {
                let angle = expected.clamped_random(360);
                let index = expected.clamped_random(5);
                let offset = expected.clamped_random(8);

                (angle, index, offset)
            })
            .collect();

        on_process(&mut state, &MasterState::default(), &mut rng);

        let slot = &state.slots[0];
        for (block, (angle, index, _)) in draws.iter().enumerate() {
            assert_eq!(slot.height[block * 4 + 3], *angle as f32, "angle block {block}");
            assert_eq!(slot.height[block * 4 + 2], *index as f32, "index block {block}");
        }

        assert_eq!(slot.vec_b_pre[1], -(3 + draws[0].2) as f32);
        assert_eq!(slot.vec_b_now[1], -(3 + draws[1].2) as f32);
        assert_eq!(slot.vec_t_pre[1], -(3 + draws[2].2) as f32);
        assert_eq!(slot.vec_t_now[1], -(3 + draws[3].2) as f32);

        // A drained block starts over at zero.
        assert_eq!(slot.height[0], 0.0);
        assert_eq!(slot.height[1], 0.0);
    }

    #[test]
    fn the_master_name_offset_lifts_the_icons() {
        let mut state = bubbles();
        let master = MasterState {
            parent_name_offset: Some(6.0),
            ..Default::default()
        };

        on_process(&mut state, &master, &mut EffectRng::new(1));

        assert_eq!(state.slots[0].height[20], 6.0);
    }

    #[test]
    fn only_live_slots_are_processed() {
        let mut state = bubbles();
        state.slots[0].life = 0;
        state.slots[1].life = 1;

        on_process(&mut state, &MasterState::default(), &mut EffectRng::new(1));

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[1].process, 1);
    }
}
