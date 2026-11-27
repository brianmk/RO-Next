//! `PP_FIRSTAID` (58) - `CEffectPrim::PrimFirstAid`,
//! `EffectPrimitiveMotion.cpp:1223-1248`; dispatch
//! `case PP_FIRSTAID:` `EffectPrimitiveMotion.cpp:9892-9893`.
//!
//! The single `_heal_effect.wav` spark panel of `CRagEffect::FirstAid`
//! (`RagEffect.cpp:5123-5161`, `EF_FIRSTAID`). `FaceCamera` sets the two
//! `Tei_TowardScreen` angles the shared `RenderFieldPanel` reads,
//! `FollowMaster` keeps the panel on the caster, and the sole slot spins its
//! `height[8]` window by 3 degrees per frame (`>= 360` wraps once), fades in by
//! `+4` for the first 20 frames and loses 1 alpha per frame after 100
//! (clamped at zero).
//!
//! Like the other slot modules this file keeps its own copy of the inline
//! `FaceCamera` (`EffectPrimitiveMotion.cpp:100-114`) so the body reads as the
//! original does. The renderer is the shared
//! [`render_field_panel`](super::super::render_slots::render_field_panel)
//! (`EffectPrimitiveRendering.cpp:3312-3505`, dispatch `:578`).

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

/// One frame of `PrimFirstAid` (`EffectPrimitiveMotion.cpp:1223-1248`).
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    face_camera(&mut state.slots, master);

    // `FollowMaster(*this)`.
    state.position = master.position;

    // `for (int ec = 0; ec < 1; ++ec)` - the original only advances slot 0.
    let slot = &mut state.slots[0];

    if slot.life != 1 {
        return;
    }

    slot.process += 1;
    slot.height[8] += 3.0;

    if slot.height[8] >= 360.0 {
        slot.height[8] -= 360.0;
    }

    if slot.process <= 20 {
        slot.alpha_b += 4;
    }

    if slot.process > 100 {
        slot.alpha_b -= 1;

        if slot.alpha_b < 0 {
            slot.alpha_b = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state() -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::FirstAid,
            duration: 299,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            process: 0,
            alpha_b: 0,
            ..Default::default()
        };
        state
    }

    #[test]
    fn follows_the_master_and_faces_the_camera() {
        let mut state = state();
        let master = MasterState {
            position: [4.0, 5.0, 6.0],
            camera_yaw: 10.0,
            camera_pitch: 20.0,
            ..Default::default()
        };

        on_process(&mut state, &master);

        assert_eq!(state.position, [4.0, 5.0, 6.0]);
        assert_eq!(state.slots[0].alpha_t, 350);
        assert_eq!(state.slots[1].alpha_t, 70);
    }

    #[test]
    fn the_window_spins_three_degrees_and_wraps() {
        let mut state = state();
        let master = MasterState::default();
        state.slots[0].height[8] = 358.0;

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].height[8], 1.0);
    }

    #[test]
    fn fades_in_for_twenty_frames() {
        let mut state = state();
        let master = MasterState::default();

        on_process(&mut state, &master);
        assert_eq!(state.slots[0].alpha_b, 4);

        state.slots[0].process = 19;
        on_process(&mut state, &master);
        assert_eq!(state.slots[0].alpha_b, 8);

        state.slots[0].process = 20;
        on_process(&mut state, &master);
        assert_eq!(state.slots[0].alpha_b, 8);
    }

    #[test]
    fn fades_out_after_one_hundred_frames() {
        let mut state = state();
        let master = MasterState::default();
        state.slots[0].process = 100;
        state.slots[0].alpha_b = 2;

        on_process(&mut state, &master);
        assert_eq!(state.slots[0].alpha_b, 1);

        state.slots[0].alpha_b = 1;
        on_process(&mut state, &master);
        assert_eq!(state.slots[0].alpha_b, 0);
    }

    #[test]
    fn a_dead_slot_does_not_advance() {
        let mut state = state();
        let master = MasterState::default();
        state.slots[0].life = 0;

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].height[8], 0.0);
        // FaceCamera still runs before the gate.
        assert_eq!(state.slots[0].alpha_t, 360);
    }
}
