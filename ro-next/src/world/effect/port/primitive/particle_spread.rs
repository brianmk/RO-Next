//! prim 115 (no `EffectPrimId.h` symbol) - `CEffectPrim::PrimParticleSpread`,
//! `EffectPrimitiveMotion.cpp:3647-3705`; dispatch `case 115:`
//! `:8793-8794`.
//!
//! The three-layer white-light burst of `CRagEffect::TexturedTumble`
//! (`EF_CARTTER`). Each live slot advances its three `vec?Pre/Now` vectors by a
//! speed derived from `maxHeight` and its stored angles, and after frame 25
//! shrinks `maxHeight` by 10% and drains `height[12..15]` by 6 per frame.
//!
//! Renderer: [`render_soul_particle`](super::super::render_slots::render_soul_particle)
//! (`RenderSoulParticle`, `EffectPrimitiveRendering.cpp:5558-5590`, dispatch
//! `:469-470`).

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

        let speed = slot.max_height * angles.raw_sine(slot.height[2] as usize);
        slot.vec_b_pre[1] -= speed;
        let speed = slot.max_height * angles.raw_cosine(slot.height[2] as usize);
        slot.vec_b_pre[0] += speed * angles.raw_sine(slot.height[0] as usize);
        slot.vec_b_pre[2] += speed * angles.raw_cosine(slot.height[0] as usize);

        let speed = slot.max_height * angles.raw_sine(slot.height[6] as usize);
        slot.vec_b_now[1] -= speed;
        let speed = slot.max_height * angles.raw_cosine(slot.height[6] as usize);
        slot.vec_b_now[0] += speed * angles.raw_sine(slot.height[4] as usize);
        slot.vec_b_now[2] += speed * angles.raw_cosine(slot.height[4] as usize);

        let speed = slot.max_height * angles.raw_sine(slot.height[10] as usize);
        slot.vec_t_pre[1] -= speed;
        let speed = slot.max_height * angles.raw_cosine(slot.height[10] as usize);
        slot.vec_t_pre[0] += speed * angles.raw_sine(slot.height[8] as usize);
        slot.vec_t_pre[2] += speed * angles.raw_cosine(slot.height[8] as usize);

        if slot.process > 25 {
            slot.max_height *= 0.9;

            for value in 12..15usize {
                slot.height[value] -= 6.0;

                if slot.height[value] < 0.0 {
                    slot.height[value] = 0.0;
                }
            }
        }
    }
}
