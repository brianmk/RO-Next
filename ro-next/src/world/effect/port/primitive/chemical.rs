//! `PP_CHEMICALPROTECTION` (54) - `CEffectPrim::PrimChemical`,
//! `EffectPrimitiveMotion.cpp:1130-1148`; dispatch
//! `case PP_CHEMICALPROTECTION:` `EffectPrimitiveMotion.cpp:9919-9920`.
//!
//! The motion behind `CRagEffect::ShockwaveRing` (`RagEffect.cpp:2807-2838`),
//! i.e. `EF_CHEMICALPROTECTION`. Each live slot fades in by `+18` for the first
//! ten frames, holds, and fades out by `-15` after frame 20 (clamped at zero).
//! It calls `FaceCamera` first, whose two angles drive `RenderChemical`'s
//! `Tei_TowardScreen`; like the other slot modules this file keeps its own copy
//! of the inline (`EffectPrimitiveMotion.cpp:100-114`) so the body reads as the
//! original does.
//!
//! The renderer is
//! [`render_chemical`](super::super::render_slots::render_chemical)
//! (`RenderChemical`, `EffectPrimitiveRendering.cpp:6906-6975`, dispatch
//! `:611-612`).

use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState};

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

/// One frame of `PrimChemical` (`EffectPrimitiveMotion.cpp:1130-1148`).
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    face_camera(&mut state.slots, master);

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process <= 10 {
            slot.alpha_b += 18;
        } else if slot.process > 20 {
            slot.alpha_b -= 15;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state(process: i32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::ChemicalProtection,
            duration: 300,
            ..Default::default()
        };
        for index in 0..4usize {
            state.slots[index] = EffectPrimitiveSlot {
                life: 1,
                process,
                alpha_b: 0,
                ..Default::default()
            };
        }
        state
    }

    #[test]
    fn face_camera_writes_the_two_slot_angles() {
        let mut state = state(0);
        let master = MasterState {
            camera_yaw: 10.0,
            camera_pitch: 20.0,
            ..Default::default()
        };

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].alpha_t, 350);
        // 360 - 20 + 90 = 430 -> 70.
        assert_eq!(state.slots[1].alpha_t, 70);
    }

    #[test]
    fn fades_in_for_ten_frames() {
        let master = MasterState::default();
        let mut state = state(0);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].process, 1);
        assert_eq!(state.slots[0].alpha_b, 18);

        state.slots[0].process = 9;
        on_process(&mut state, &master);
        assert_eq!(state.slots[0].alpha_b, 36);
    }

    #[test]
    fn holds_between_eleven_and_twenty() {
        let master = MasterState::default();
        let mut state = state(10);
        state.slots[0].alpha_b = 180;

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].process, 11);
        assert_eq!(state.slots[0].alpha_b, 180);
    }

    #[test]
    fn fades_out_after_twenty_and_clamps() {
        let master = MasterState::default();
        let mut state = state(20);
        state.slots[0].alpha_b = 20;

        on_process(&mut state, &master);
        assert_eq!(state.slots[0].alpha_b, 5);

        state.slots[0].alpha_b = 10;
        on_process(&mut state, &master);
        assert_eq!(state.slots[0].alpha_b, 0);
    }

    #[test]
    fn dead_slots_keep_their_alpha() {
        let master = MasterState::default();
        let mut state = state(0);
        state.slots[2].life = 0;
        state.slots[2].alpha_b = 99;

        on_process(&mut state, &master);

        assert_eq!(state.slots[2].process, 0);
        assert_eq!(state.slots[2].alpha_b, 99);
    }
}
