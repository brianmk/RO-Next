//! prim 41 (no `EffectPrimId.h` symbol) - the inline `case 41:`
//! (`EffectPrimitiveMotion.cpp:9036-9089`).
//!
//! Spawned only by `CRagEffect::TexturedCross` (`EF_PROVIDENCE`). Each live
//! slot advances one degree per frame; after 200 frames the 21 heights rise
//! through `sin(process - 200) * maxHeight`, and from frame 800 `maxHeight`
//! bleeds down to zero. `alphaT == 1` (the body sets it) fades `alphaB` out
//! after frame 250.
//!
//! The renderer is the shared
//! [`render_3d_casting`](super::super::render_slots::render_3d_casting)
//! (`Render3DCasting`, `EffectPrimitiveRendering.cpp:6596-6675`, dispatch
//! `:504`).

use super::{Angles, EffectPrimitiveState, MasterState};

/// One frame of the inline `case 41` (`EffectPrimitiveMotion.cpp:9036-9089`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;
        slot.rot_start += 1;

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        if slot.alpha_t == 1 && slot.process > 250 {
            slot.alpha_b -= 1;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }

        if slot.process >= 800 {
            slot.max_height -= 1.0;

            if slot.max_height < 0.0 {
                slot.max_height = 0.0;
            }
        }

        for value in 0..21usize {
            slot.height[value] = if slot.process < 200 {
                0.0
            } else if slot.process <= 290 {
                angles.raw_sine((slot.process - 200) as usize) * slot.max_height
            } else {
                slot.max_height
            };

            if slot.height[value] < 0.0 {
                slot.height[value] = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::TexturedCross,
            duration: 200,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: 0,
            max_height: 120.0,
            alpha_b: 40,
            alpha_t: 1,
            full_display_angle: 360,
            ..Default::default()
        };
        state
    }

    #[test]
    fn heights_stay_flat_until_frame_two_hundred() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 198;

        on_process(&mut state, &angles, &MasterState::default());

        assert_eq!(state.slots[0].process, 199);
        assert!(state.slots[0].height.iter().all(|height| *height == 0.0));
    }

    #[test]
    fn heights_follow_the_sine_between_two_hundred_and_two_ninety() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 204;

        on_process(&mut state, &angles, &MasterState::default());

        let expected = angles.raw_sine(5) * 120.0;
        assert!((state.slots[0].height[0] - expected).abs() < 1e-6);
    }

    #[test]
    fn alpha_fades_after_two_hundred_fifty() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 250;
        state.slots[0].alpha_b = 1;

        on_process(&mut state, &angles, &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 0);
    }

    #[test]
    fn max_height_bleeds_after_eight_hundred() {
        let angles = Angles::new();
        let mut state = state();
        state.slots[0].process = 800;
        state.slots[0].max_height = 0.5;

        on_process(&mut state, &angles, &MasterState::default());

        assert_eq!(state.slots[0].max_height, 0.0);
        assert_eq!(state.slots[0].height[0], 0.0);
    }
}
