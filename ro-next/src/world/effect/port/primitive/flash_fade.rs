//! prim 154 (`PrimFlashFade`) - `CEffectPrim::PrimFlashFade`,
//! `EffectPrimitiveMotion.cpp:5095-5119`; dispatch at
//! `EffectPrimitiveMotion.cpp:9928-9930`.
//!
//! The four slots fade in by 13 alpha per frame for their first 15 frames,
//! hold for frames 16-20, then fade out by 3 and clear `life` as soon as the
//! alpha would go negative. `PrimFlashFade` also runs the file-local
//! `FaceCamera` inline (`EffectPrimitiveMotion.cpp:100-114`), which stores the
//! camera yaw/pitch in `slots[0].alphaT`/`slots[1].alphaT`.
//!
//! The official id has no symbol in `EffectPrimId.h` (it stops at 104); the
//! port names the primitive after its motion function. Its renderer is
//! `RenderBladeFins` (`EffectPrimitiveArcs.cpp:1394`), ported in
//! `render_slots::render_blade_fins`; `CRagEffect::TexturedBeam` is the body
//! that spawns it (`RagEffect.cpp:3043-3079`).

use super::{EffectPrimitiveSlot, EffectPrimitiveState, MasterState};

/// One frame of prim 154; `PrimFlashFade` returns `void`, so the primitive
/// only retires through the shared `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    face_camera(&mut state.slots, master);

    for slot in &mut state.slots {
        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process <= 15 {
            slot.alpha_b = (slot.alpha_b + 13).min(255);
        } else if slot.process > 20 {
            slot.alpha_b -= 3;

            if slot.alpha_b < 0 {
                slot.life = 0;
                slot.alpha_b = 0;
            }
        }
    }
}

/// `FaceCamera(EffectPrimitiveSlot*)` - `EffectPrimitiveMotion.cpp:100-114`.
///
/// Same body as the one in [`super::wandering_flames`] (the inline lives in
/// both translation units' call sites); kept local with its own tests so the
/// two primitive ports stay independent.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;

    fn state() -> EffectPrimitiveState {
        EffectPrimitiveState {
            count: PrimitiveId::FlashFade,
            duration: 300,
            ..Default::default()
        }
    }

    #[test]
    fn primitive_id_is_154() {
        assert_eq!(PrimitiveId::FlashFade.id(), 154);
    }

    #[test]
    fn face_camera_writes_the_camera_angles() {
        let master = MasterState {
            camera_yaw: 30.0,
            camera_pitch: 20.0,
            ..Default::default()
        };
        let mut state = state();

        on_process(&mut state, &master);

        // FaceCamera has no slot-life gate.
        assert_eq!(state.slots[0].alpha_t, 330);
        assert_eq!(state.slots[1].alpha_t, 70);
    }

    #[test]
    fn face_camera_keeps_the_unwrapped_360_on_slot_zero() {
        let mut state = state();

        on_process(&mut state, &MasterState::default());

        // `360 - 0` is assigned to an int16 without a >= 360 fixup.
        assert_eq!(state.slots[0].alpha_t, 360);
        // Slot one is fixed up: 360 - 0 + 90 = 450 -> 90.
        assert_eq!(state.slots[1].alpha_t, 90);
    }

    #[test]
    fn first_fifteen_frames_fade_in_by_thirteen() {
        let mut state = state();
        state.slots[0].life = 1;

        for frame in 1..=15 {
            on_process(&mut state, &MasterState::default());

            assert_eq!(state.slots[0].process, frame);
            assert_eq!(state.slots[0].alpha_b, frame as i16 * 13);
        }

        // 15 * 13 = 195, below the 255 clamp.
        assert_eq!(state.slots[0].alpha_b, 195);
    }

    #[test]
    fn fade_in_clamps_at_255() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].alpha_b = 250;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].alpha_b, 255);
    }

    #[test]
    fn frames_sixteen_to_twenty_hold_the_alpha() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].process = 15;
        state.slots[0].alpha_b = 195;

        for _ in 0..5 {
            on_process(&mut state, &MasterState::default());
        }

        assert_eq!(state.slots[0].process, 20);
        assert_eq!(state.slots[0].alpha_b, 195);
    }

    #[test]
    fn fading_out_clears_the_slot_when_the_alpha_goes_negative() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].process = 20;
        state.slots[0].alpha_b = 1;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].process, 21);
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 0);
    }

    #[test]
    fn fading_out_steps_by_three_until_it_reaches_zero() {
        let mut state = state();
        state.slots[0].life = 1;
        state.slots[0].process = 20;
        state.slots[0].alpha_b = 7;

        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 4);
        assert_eq!(state.slots[0].life, 1);

        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 1);
        assert_eq!(state.slots[0].life, 1);

        on_process(&mut state, &MasterState::default());
        assert_eq!(state.slots[0].alpha_b, 0);
        assert_eq!(state.slots[0].life, 0);
    }

    #[test]
    fn dead_slots_are_not_touched() {
        let mut state = state();
        state.slots[1].process = 9;
        state.slots[1].alpha_b = 40;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[1].process, 9);
        assert_eq!(state.slots[1].alpha_b, 40);
    }
}
