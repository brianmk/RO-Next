//! `PP_SPHEREWIND` (77) - the inline `case PP_SPHEREWIND:`
//! (`EffectPrimitiveMotion.cpp:8657-8677`).
//!
//! Spawned by `CRagEffect::FeatherRing` (`EF_SPHEREWIND2`): each live slot
//! advances `rotStart` by `height[1]` (truncated through `int16_t`, exactly
//! like the original) and wraps once at 360.
//!
//! The renderer is the shared
//! [`render_sphere_wind`](super::super::render_slots::render_sphere_wind)
//! (`RenderSphereWind`, `EffectPrimitiveRendering.cpp:5591-5646`, dispatch
//! `:404-405`), which prim 91 also uses.

use super::{EffectPrimitiveState, MasterState};

/// One frame of `case PP_SPHEREWIND` (`EffectPrimitiveMotion.cpp:8657-8677`).
pub fn on_process(state: &mut EffectPrimitiveState, master: &MasterState) {
    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;
        slot.rot_start = (slot.rot_start as f32 + slot.height[1]) as i16;

        if slot.rot_start >= 360 {
            slot.rot_start -= 360;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    #[test]
    fn rot_start_advances_by_height_one_and_wraps() {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::SphereWind,
            duration: 100,
            ..Default::default()
        };
        state.slots[0] = EffectPrimitiveSlot {
            life: 1,
            rot_start: 359,
            height: [0.0; 21],
            ..Default::default()
        };
        state.slots[0].height[1] = 4.5;

        let master = MasterState {
            position: [1.0, 2.0, 3.0],
            ..Default::default()
        };

        on_process(&mut state, &master);

        assert_eq!(state.position, [1.0, 2.0, 3.0]);
        assert_eq!(state.slots[0].process, 1);
        // 359 + 4.5 = 363.5 -> 363 -> 3.
        assert_eq!(state.slots[0].rot_start, 3);
    }

    #[test]
    fn dead_slots_are_left_alone() {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::SphereWind,
            duration: 100,
            ..Default::default()
        };
        state.slots[0].life = 0;
        state.slots[0].rot_start = 10;

        on_process(&mut state, &MasterState::default());

        assert_eq!(state.slots[0].process, 0);
        assert_eq!(state.slots[0].rot_start, 10);
    }
}
