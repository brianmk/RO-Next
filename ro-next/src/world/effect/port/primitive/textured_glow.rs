//! prim 50 - the inline `case 50` motion of `CRagEffect::TexturedGlow`,
//! `EffectPrimitiveMotion.cpp:8962-8994` (`EffectPrimitiveMotion.cpp:8962`).
//!
//! `EffectPrimId.h` has no symbol for 50 (it jumps from `PP_BOTTOMOUT` = 49 to
//! `PP_POKJUK` = 52), so the port names the variant after its only spawner:
//! `CRagEffect::TexturedGlow` (`RagEffect.cpp:5516-5565`), the warp glow of
//! `EF_BOTTOM_ETERNALCHAOS` and `EF_BOTTOM_SIEGFRIED`.
//!
//! The motion is fully inline in the `switch`, unlike the named
//! `CEffectPrim::Prim*` bodies:
//!
//! * the primitive anchors to the master every frame,
//! * each live slot advances `process`, sweeps `alphaT` once per frame
//!   (`distance = sin(alphaT) + 3`) and turns `rotStart` by 3 degrees,
//! * all 21 `height[]` samples are rebuilt from the sine table: the base
//!   `sin(sample * 9) * maxHeight` plus `30%` of it modulated by `sin(angle %
//!   360)`, where `angle` is `process + index * 90` for the first two slots and
//!   `(process + index * 45) * 2` for the other two.
//!
//! Only slot 0 is armed by `TexturedGlow`; the other three are left dead by
//! the body (`life = 0`), so the loop usually has a single live iteration.
//! The renderer is the shared
//! [`render_3d_casting`](super::super::render_slots::render_3d_casting)
//! (`Render3DCasting`, `EffectPrimitiveRendering.cpp:6596-6675`, dispatch
//! `:504`).

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of the inline prim 50 motion (`EffectPrimitiveMotion.cpp:8962`).
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;
        slot.alpha_t += 1;

        if slot.alpha_t >= 360 {
            slot.alpha_t -= 360;
        }

        slot.distance = angles.raw_sine(slot.alpha_t as usize) + 3.0;

        slot.rot_start += 3;

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        for value in 0..21usize {
            // `const int phase = index * 45;`
            let phase = (index * 45) as i32;
            // `const int angle = phase < 90 ? process + index * 90
            //  : (process + phase) * 2;`
            let angle = if phase < 90 {
                slot.process + (index * 90) as i32
            } else {
                (slot.process + phase) * 2
            };
            let sine = angles.raw_sine(value * 9);

            slot.height[value] = sine * slot.max_height;
            slot.height[value] += sine * slot.max_height * 0.3 * angles.raw_sine((angle % 360) as usize);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::TexturedGlow,
            duration: 29999,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: 0,
            alpha_t: 0,
            rot_start: 0,
            max_height: 30.0,
            distance: 0.0,
            ..Default::default()
        };
        state
    }

    #[test]
    fn follows_the_master_and_advances_the_slot() {
        let angles = Angles::new();
        let master = MasterState {
            position: [7.0, 8.0, 9.0],
            ..Default::default()
        };
        let mut state = state();

        on_process(&mut state, &angles, &master);

        assert_eq!(state.position, [7.0, 8.0, 9.0]);
        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].alpha_t, 1);
        assert_eq!(state.slots[0].distance, angles.raw_sine(1) + 3.0);
        assert_eq!(state.slots[0].rot_start, 3);
    }

    #[test]
    fn heights_use_the_sine_table_with_the_index_phase_rule() {
        let angles = Angles::new();
        let master = MasterState {
            position: [1.0, 2.0, 3.0],
            ..Default::default()
        };
        let mut state = state();

        on_process(&mut state, &angles, &master);

        // Slot 0, first sample: sin(0) * 30 + sin(0) * 30 * 0.3 * sin(1 % 360).
        let expected0 = angles.raw_sine(0) * 30.0 + angles.raw_sine(0) * 30.0 * 0.3 * angles.raw_sine(1);
        assert!((state.slots[0].height[0] - expected0).abs() < 1e-6);

        // Slot 0, sample 1: sin(9) * 30 * (1 + 0.3 * sin(1)).
        let expected1 = angles.raw_sine(9) * 30.0 + angles.raw_sine(9) * 30.0 * 0.3 * angles.raw_sine(1);
        assert!((state.slots[0].height[1] - expected1).abs() < 1e-6);
    }

    #[test]
    fn only_live_slots_move() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = state();
        state.slots[1].life = 0;

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[1].process, 0);
        assert_eq!(state.slots[1].alpha_t, 0);
        assert_eq!(state.slots[1].distance, 0.0);
    }

    #[test]
    fn angle_alpha_and_rotation_wrap_at_three_sixty() {
        let angles = Angles::new();
        let master = MasterState::default();
        let mut state = state();
        state.slots[0].alpha_t = 359;
        state.slots[0].rot_start = 358;

        on_process(&mut state, &angles, &master);

        assert_eq!(state.slots[0].alpha_t, 0);
        assert_eq!(state.slots[0].rot_start, 1);
        assert_eq!(state.slots[0].distance, angles.raw_sine(0) + 3.0);
    }
}
