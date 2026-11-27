//! `PP_CHEMICAL3` (56) - `CEffectPrim::PrimChemical3`,
//! `EffectPrimitiveMotion.cpp:1084-1117`; dispatch
//! `case PP_CHEMICAL3:` `EffectPrimitiveMotion.cpp:9925-9926`.
//!
//! The line streaks of `CRagEffect::LineStreakLayered` (`EF_CHEMICAL3` /
//! `EF_CHEMICAL4`). Each live slot fades in (`+12` for the blue textures,
//! `+20` for the green ones) for ten frames; the `flag1[0] >= 10` branch also
//! drifts `height[3]` and therefore raises `height[1]`, wrapping at 200. After
//! twenty frames it fades out by 4 (blue) or 2 (green).
//!
//! Renderer: [`render_ground_bar`](super::super::render_slots::render_ground_bar)
//! (`RenderGroundBar`, `EffectPrimitiveRendering.cpp:3925-4030`, dispatch
//! `:638-639`).

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

/// One frame of `PrimChemical3` (`EffectPrimitiveMotion.cpp:1084-1117`).
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    face_camera(&mut state.slots, master);

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process <= 10 {
            if slot.flag1[0] < 10 {
                slot.alpha_b += 12;
            } else {
                slot.alpha_b += 20;
            }
        }

        if slot.flag1[0] >= 10 {
            slot.height[3] += 0.05;
        }

        slot.height[1] += slot.height[3];

        if slot.height[1] >= 200.0 {
            slot.height[1] -= 200.0;
        }

        if slot.process > 20 {
            if slot.flag1[0] < 10 {
                slot.alpha_b -= 4;
            } else {
                slot.alpha_b -= 2;
            }

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

    fn state(flag: i8) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Chemical3,
            duration: 300,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            flag1: [flag; 21],
            ..Default::default()
        };
        state
    }

    #[test]
    fn fades_in_then_out_for_the_blue_textures() {
        let master = MasterState::default();
        let mut state = state(0);

        on_process(&mut state, &master);
        assert_eq!(state.slots[0].alpha_b, 12);
        assert_eq!(state.slots[0].height[1], 0.0);

        state.slots[0].process = 20;
        state.slots[0].alpha_b = 50;
        on_process(&mut state, &master);
        assert_eq!(state.slots[0].alpha_b, 46);
    }

    #[test]
    fn green_textures_fade_faster_and_rise() {
        let master = MasterState::default();
        let mut state = state(20);

        on_process(&mut state, &master);

        assert_eq!(state.slots[0].alpha_b, 20);
        // `height[3]` starts at zero and gains 0.05 before the lift.
        assert!((state.slots[0].height[3] - 0.05).abs() < 1e-6);
        assert!((state.slots[0].height[1] - 0.05).abs() < 1e-6);
    }

    #[test]
    fn the_height_wraps_at_two_hundred() {
        let master = MasterState::default();
        let mut state = state(20);
        state.slots[0].height[1] = 199.99;

        on_process(&mut state, &master);

        assert!((state.slots[0].height[1] - 0.04).abs() < 1e-5);
    }
}
