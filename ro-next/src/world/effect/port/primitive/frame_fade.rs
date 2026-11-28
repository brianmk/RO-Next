//! Prim 140 (`PrimFrameFade`) - `EffectPrimitiveMotion.cpp:4829-4853`.
//!
//! The scrolling pillar used by `EF_TETRA*`. Each live slot advances every
//! frame; every third frame it steps the texture frame (`flag1[0]`) and, once
//! the last frame is reached, keeps it there and fades `alphaB` by 15.

use super::{EffectPrimitiveState, MasterState};

/// `FaceCamera(EffectPrimitiveSlot*)` for the renderer's two angles.
fn face_camera(state: &mut EffectPrimitiveState, master: &MasterState) {
    let yaw = master.camera_yaw as i16 as i32;
    state.slots[0].alpha_t = (360 - yaw) as i16;

    let pitch = master.camera_pitch as i16 as i32;
    let mut alpha_t = 360 - pitch + 90;
    if alpha_t >= 360 {
        alpha_t -= 360;
    }
    state.slots[1].alpha_t = alpha_t as i16;
}

pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    face_camera(state, master);

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process % 3 == 0 {
            slot.flag1[0] += 1;

            let frames = state.total_texture as i32;
            if slot.flag1[0] as i32 >= frames {
                slot.flag1[0] = (frames - 1) as i8;
                slot.alpha_b -= 15;
            }
        }

        if slot.alpha_b > 255 {
            slot.alpha_b = 255;
        } else if slot.alpha_b < 0 {
            slot.alpha_b = 0;
        }
    }
}
