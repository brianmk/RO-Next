//! `PP_ACIDDEMON` (118) - `CEffectPrim::PrimAcidDemon`,
//! `EffectPrimitiveMotion.cpp:3161-3210`; dispatch `:8374-8375`.
//!
//! `TexturedFireWall` arms four columns at `distance` 2.2..3.0 and
//! `maxHeight` 19..30. This motion rotates each column, fades it in for twenty
//! frames, holds through frame forty, then fades out by fifteen, and rebuilds
//! the 21 height samples from the sine table every frame. Columns 0..2 grow
//! by 1.005 and the fourth by 1.025 per frame. A sample that exceeds its
//! sine-scaled maximum sets `flag1[i]`, exactly like the original.
//!
//! The renderer is the shared `Render3DCasting` (`EffectPrimitiveRendering.cpp`
//! `:512`), so the same casting primitive geometry is used.

use super::super::angles::Angles;
use super::super::runtime::EffectRng;
use super::{EffectPrimitiveState, MasterState};

pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState, rng: &mut EffectRng) {
    state.position = master.position;

    const MIDDLE: i32 = (21 - 1) / 2;
    const M2: i32 = 90 / MIDDLE;

    for ec in 0..4usize {
        if state.slots[ec].life != 1 {
            continue;
        }

        let slot = &mut state.slots[ec];
        slot.process += 1;

        let step = if ec == 3 {
            2 + rng.clamped_random(2)
        } else {
            4 + rng.clamped_random(3)
        };
        slot.rot_start += step as i16;

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }

        if slot.process >= 40 {
            slot.alpha_b -= 15;

            if slot.alpha_b < 0 {
                slot.alpha_b = 0;
            }
        } else if slot.process < 20 {
            slot.alpha_b += 5;

            if slot.alpha_b > 80 {
                slot.alpha_b = 80;
            }
        }

        slot.max_height *= if ec == 3 { 1.025 } else { 1.005 };

        for i in 0..21usize {
            let sin_limit = 90 + (i as i32 - MIDDLE) * M2;

            if slot.process <= 90 {
                slot.height[i] = angles.sine(sin_limit) * slot.max_height * angles.sine(slot.process);
            } else {
                slot.flag1[i] = 1;
            }

            if slot.height[i] > angles.sine(sin_limit) * slot.max_height {
                slot.flag1[i] = 1;
            }

            if slot.height[i] < 0.0 {
                slot.height[i] = 0.0;
            }
        }
    }
}
