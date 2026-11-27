//! prim 73 `PP_MAGNUM2` - the bare `case PP_MAGNUM2:` in
//! `EffectPrimitiveMotion.cpp:8377-8420`; dispatch `:8377`.
//!
//! The yellow shockwave ring of `EF_MAGNUM2` (339), spawned by
//! `CRagEffect::Magnum2` (`RagEffect.cpp:5197-5242`). The effect body launches
//! one primitive per three-frame step with two live slots; this motion is a
//! pure fade:
//!
//! ```cpp
//! ++slot.process;
//! if (slot.riseAngle < 0) slot.riseAngle = 0;
//! if (slot.rotStart >= 360) slot.rotStart -= 360;
//! if (slot.process < 9) {
//!   if (slot.alphaT == 0) { slot.alphaB += 10; if (slot.alphaB > 80) slot.alphaB = 80; }
//!   else                  { slot.alphaB +=  5; if (slot.alphaB > 40) slot.alphaB = 40; }
//! } else {
//!   slot.alphaB -= 30;
//!   if (slot.alphaB < 0) { slot.alphaB = 0; slot.life = 0; }
//! }
//! ```
//!
//! `alphaT` is the slot index (`0`/`1`) the body writes, so the second slot
//! rises to `40` while the first reaches `80`.
//!
//! The renderer is `RenderMagnum2` (`EffectPrimitiveRendering.cpp:2121`),
//! dispatched by the `case PP_MAGNUM2` arm at
//! `EffectPrimitiveRendering.cpp:388-390` - see
//! [`super::super::render_slots::render_magnum_2`].

use super::{EffectPrimitiveState, MasterState};

/// One frame of the official `case PP_MAGNUM2:`
/// (`EffectPrimitiveMotion.cpp:8377-8420`).
///
/// The motion is `void`, so the primitive retires through the shared
/// `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    // `const vector3d& parent = this->state.master->render.position;`
    // `this->render.position = parent;`
    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.rise_angle < 0 {
            slot.rise_angle = 0;
        }

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        if slot.process < 9 {
            if slot.alpha_t == 0 {
                slot.alpha_b += 10;

                if slot.alpha_b > 80 {
                    slot.alpha_b = 80;
                }
            } else {
                slot.alpha_b += 5;

                if slot.alpha_b > 40 {
                    slot.alpha_b = 40;
                }
            }
        } else {
            slot.alpha_b -= 30;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
                slot.life = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn wave() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Magnum2,
            duration: 100,
            size: 5.0,
            ..Default::default()
        };

        for index in 0..2usize {
            let slot = &mut state.slots[index];
            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 360;
            slot.max_height = 20.0;
            slot.alpha_b = 0;
            slot.distance = 5.0;
            slot.rise_angle = 70 - index as i16 * 8;
            slot.alpha_t = index as i16;
        }

        state
    }

    #[test]
    fn magnum_two_is_primitive_73() {
        assert_eq!(PrimitiveId::Magnum2.id(), 73);
    }

    #[test]
    fn the_two_slots_ramp_to_their_caps() {
        let mut state = wave();

        // Nine positive frames: 0 -> 80 for slot 0, 0 -> 40 for slot 1.
        for _ in 0..8 {
            on_process(&mut state, &MasterState::default());
        }

        assert_eq!(state.slots[0].process, 8);
        assert_eq!(state.slots[0].alpha_b, 80);
        assert_eq!(state.slots[1].alpha_b, 40);
        assert_eq!(state.slots[0].life, 1);
    }

    #[test]
    fn frame_nine_drains_thirty_and_kills_the_slot_below_zero() {
        let mut state = wave();
        state.slots[0].process = 8;
        state.slots[1].process = 8;
        state.slots[0].alpha_b = 80;
        state.slots[1].alpha_b = 40;

        // process becomes 9: the drain branch runs.
        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 50);
        assert_eq!(state.slots[1].alpha_b, 10);
        assert_eq!(state.slots[0].life, 1);

        on_process(&mut state, &MasterState::default());

        // Slot 1 started at 10, so this frame kills it; slot 0 is still alive.
        assert_eq!(state.slots[0].process, 10);
        assert_eq!(state.slots[0].alpha_b, 20);
        assert_eq!(state.slots[0].life, 1);
        assert_eq!(state.slots[1].process, 10);
        assert_eq!(state.slots[1].alpha_b, 0);
        assert_eq!(state.slots[1].life, 0);

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].process, 11);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 0);
    }

    #[test]
    fn the_first_frame_normalizes_the_angles() {
        let mut state = wave();
        state.slots[0].rot_start = 360;
        state.slots[1].rise_angle = -4;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].rot_start, 0);
        assert_eq!(state.slots[1].rise_angle, 0);
    }

    #[test]
    fn the_motion_follows_the_master() {
        let mut state = wave();

        on_process(&mut state, &MasterState {
            position: [4.0, 5.0, 6.0],
            ..Default::default()
        });

        assert_eq!(state.position, [4.0, 5.0, 6.0]);
    }
}
