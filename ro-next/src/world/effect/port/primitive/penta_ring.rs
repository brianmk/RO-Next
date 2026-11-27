//! Prim 69 - the `EF_TRIPLEATTACK2`/`EF_TRIPLEATTACK3` cloud primitive.
//!
//! `EffectPrimId.h` has no symbol between `PP_ASURACASTING` (68) and
//! `PP_HITLINE` (70); the official `OnProcess` switch handles it with a bare
//! `case 69` (`EffectPrimitiveMotion.cpp:9592-9639`) and
//! `CEffectPrim::Render` draws it with `RenderPentaRing`
//! (`EffectPrimitiveRendering.cpp:693-695`, body `:1399-1477`). The port names
//! the variant after that renderer.
//!
//! The motion is a pure slot alpha ramp plus the rotating `vecTNow` ring
//! position; it never follows the master and never calls `ProcessAlpha`, so
//! `state.alphaB` (not `state.alpha`) is what the renderer consumes.
//! `height[4]` selects the branch exactly as in the original, including the
//! distinct `5.0` / `2.0` values the three bodies write.

use super::super::angles::Angles;
use super::EffectPrimitiveState;

/// One frame of the official `case 69`
/// (`EffectPrimitiveMotion.cpp:9592-9639`).
pub fn on_process(state: &mut EffectPrimitiveState) -> bool {
    let angles = Angles::shared();

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        // `if (slot.life <= 0) continue;`
        if slot.life <= 0 {
            continue;
        }

        // `++slot.process; if (slot.process <= 0) continue;`
        slot.process += 1;

        if slot.process <= 0 {
            continue;
        }

        // `if (slot.height[4] == 5.0f) { ... } else if (slot.process < 6) {
        //  ... } else { ... }`
        if slot.height[4] == 5.0 {
            if slot.process > 20 {
                slot.alpha_b -= 2;

                if slot.alpha_b < 0 {
                    slot.alpha_b = 0;
                }
            } else {
                slot.alpha_b += 8;
            }
        } else if slot.process < 6 {
            if slot.height[4] == 2.0 {
                slot.alpha_b += 2;
            } else {
                slot.alpha_b += 8;
            }
        } else {
            if slot.height[4] == 2.0 {
                slot.alpha_b -= 1;
            } else {
                slot.alpha_b -= 4;
            }

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }

        // `slot.vecTNow.x = slot.distance * g_cosTable[slot.rotStart] +
        //  slot.vecBNow.x;` (the y component is left untouched; the bodies seed
        //  it with the effect height or the `RenderHitLine3` sample count).
        slot.vec_t_now[0] = slot.distance * angles.raw_cosine(slot.rot_start.max(0) as usize) + slot.vec_b_now[0];
        slot.vec_t_now[2] = slot.distance * angles.raw_sine(slot.rot_start.max(0) as usize) + slot.vec_b_now[2];
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveState, PrimitiveId};

    fn state(kind: f32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::PentaRing,
            duration: 100,
            ..Default::default()
        };

        state.slots[0].life = 1;
        state.slots[0].height[4] = kind;
        state.slots[0].rot_start = 0;
        state.slots[0].distance = 20.0;
        state.slots[0].vec_b_now = [3.0, 4.0, 5.0];

        state
    }

    #[test]
    fn alpha_ramps_up_then_down_for_kind_two() {
        let mut state = state(2.0);

        // process 1..5 += 2, then -1 and clamped at zero.
        for _ in 0..5 {
            assert!(on_process(&mut state));
        }
        assert_eq!(state.slots[0].alpha_b, 10);

        for _ in 0..6 {
            assert!(on_process(&mut state));
        }
        assert_eq!(state.slots[0].alpha_b, 4);

        for _ in 0..10 {
            assert!(on_process(&mut state));
        }
        assert_eq!(state.slots[0].alpha_b, 0);
    }

    #[test]
    fn kind_five_decays_after_frame_twenty() {
        let mut state = state(5.0);

        for _ in 0..20 {
            assert!(on_process(&mut state));
        }
        assert_eq!(state.slots[0].alpha_b, 160);

        assert!(on_process(&mut state));
        assert_eq!(state.slots[0].alpha_b, 158);
    }

    #[test]
    fn ring_position_rotates_from_the_binary_anchor() {
        let mut state = state(0.0);

        assert!(on_process(&mut state));

        let angles = Angles::shared();
        assert_eq!(
            state.slots[0].vec_t_now[0],
            state.slots[0].distance * angles.raw_cosine(0) + state.slots[0].vec_b_now[0]
        );
        assert_eq!(
            state.slots[0].vec_t_now[2],
            state.slots[0].distance * angles.raw_sine(0) + state.slots[0].vec_b_now[2]
        );
        // The `y` component is only ever written by the body.
        assert_eq!(state.slots[0].vec_t_now[1], 0.0);
    }

    #[test]
    fn a_dead_or_negative_process_slot_is_left_alone() {
        let mut state = state(0.0);
        state.slots[0].process = -1;
        on_process(&mut state);
        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].vec_t_now, [0.0, 0.0, 0.0]);

        state.slots[0].life = 0;
        on_process(&mut state);
        assert_eq!(state.slots[0].process, 0);
    }
}
