//! prim 88 - `CEffectPrim::PrimGrowFollow`,
//! `EffectPrimitiveMotion.cpp:5264-5295`; dispatch at
//! `EffectPrimitiveMotion.cpp:8781-8782`.
//!
//! The ground ring of `EF_PRESSURE` (365), spawned by
//! `CRagEffect::TexturedLevelRing` (`RagEffect.cpp:4096-4140`).
//! `EffectPrimId.h` has no symbol for it, so the port names the variant after
//! its motion body.
//!
//! Every live slot follows the master, grows its radius by 7% per frame and
//! ramps its alpha by `+2` per frame for ten frames (by `+6` when the
//! primitive's `size` is `113`, a level-4 `TexturedLevelRing`). From frame 11
//! on it drains one alpha per frame; the drain is short-circuited behind
//! `process > 10`, so a slot that is exactly on frame 10 still grows.
//!
//! The renderer is the shared `RenderSizedSquare`
//! (`EffectPrimitiveRendering.cpp:5942`), dispatched by the `case 88` arm at
//! `EffectPrimitiveRendering.cpp:417-418`.

use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimGrowFollow` (`EffectPrimitiveMotion.cpp:5264-5295`).
///
/// The motion is `void`, so the primitive retires through the shared
/// `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    // `const CRagEffect* master = this->state.master;`
    // `this->render.position = master->render.position;`
    state.position = master.position;

    let size = state.size;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life <= 0 {
            continue;
        }

        slot.process += 1;

        if slot.process <= 0 {
            continue;
        }

        if slot.process <= 10 {
            if size == 113.0 {
                slot.alpha_b += 6;
            } else {
                slot.alpha_b += 2;
            }
        }

        slot.distance *= 1.07;

        // `if (process > 10 && --alphaB < 0) alphaB = 0;` - the decrement is
        // inside the short-circuit, so it only runs after frame 10.
        if slot.process > 10 {
            slot.alpha_b -= 1;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn level_ring() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::GrowFollow,
            duration: 300,
            size: 1.0,
            ..Default::default()
        };
        state.slots[0].life = 1;
        state.slots[0].distance = 15.0;
        state.slots[0].alpha_b = 0;

        state
    }

    #[test]
    fn grow_follow_is_primitive_88() {
        assert_eq!(PrimitiveId::GrowFollow.id(), 88);
    }

    #[test]
    fn the_ring_grows_and_ramps_two_alpha_per_frame() {
        let mut state = level_ring();
        state.position = [0.0; 3];

        on_process(&mut state, &MasterState {
            position: [1.0, 2.0, 3.0],
            ..Default::default()
        });

        let slot = &state.slots[0];
        assert_eq!(state.position, [1.0, 2.0, 3.0]);
        assert_eq!(slot.process, 1);
        assert_eq!(slot.alpha_b, 2);
        assert!((slot.distance - 15.0 * 1.07).abs() < 1e-5);
    }

    #[test]
    fn the_level_four_ring_ramps_six() {
        let mut state = level_ring();
        state.size = 113.0;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 6);
    }

    #[test]
    fn the_drain_starts_after_frame_ten() {
        let mut state = level_ring();
        state.slots[0].process = 10 - 1;
        state.slots[0].alpha_b = 20;

        // Frame 10 still ramps.
        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[0].process, 10);
        assert_eq!(state.slots[0].alpha_b, 22);

        // Frame 11 drains.
        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[0].process, 11);
        assert_eq!(state.slots[0].alpha_b, 21);
    }

    #[test]
    fn dead_slots_are_skipped() {
        let mut state = level_ring();
        state.slots[0].life = 0;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].distance, 15.0);
    }
}
