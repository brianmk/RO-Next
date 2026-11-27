//! `PP_SOULLINK` (109) - the inline `case PP_SOULLINK:`
//! (`EffectPrimitiveMotion.cpp:8808-8843`).
//!
//! The light orbit around the caster of `CRagEffect::TexturedOrbit`
//! (`EF_SOULLINK`). Each live slot sweeps `vecTNow.x` forward by
//! `process / 30` degrees per frame (capped at 359), grows `height[13]` by 20
//! for ten frames and bleeds it by 5 after 170. `vecBPre` is the slot's
//! position on the orbit.
//!
//! Renderer: [`render_soul_link`](super::super::render_slots::render_soul_link)
//! (`RenderSoulLink`, `EffectPrimitiveRendering.cpp:5469-5557`, dispatch
//! `:479-480`).

use super::{Angles, EffectPrimitiveSlot, EffectPrimitiveState, MasterState};

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

pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) {
    face_camera(&mut state.slots, master);
    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.process <= 0 {
            continue;
        }

        if slot.process <= 10 {
            slot.height[13] += 20.0;
        } else if slot.process > 170 {
            slot.height[13] -= 5.0;

            if slot.height[13] < 0.0 {
                slot.height[13] = 0.0;
            }
        }

        let mut angle = slot.process as f32 / 10.0 / 3.0 + slot.vec_t_now[0];
        slot.vec_t_now[0] = angle;

        if angle >= 359.0 {
            slot.vec_t_now[0] = 359.0;
            angle = 359.0;
        }

        slot.vec_b_pre[0] = (angle as i32) as f32 / 6.0 - 30.0;
        slot.vec_b_pre[1] = -(angles.raw_sine(angle as usize) * 20.0) - 40.0;
        slot.vec_b_pre[2] = 0.0;
    }
}
