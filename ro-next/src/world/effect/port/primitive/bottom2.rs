//! prim 48 (`PP_BOTTOM2`) - `CEffectPrim::PrimBottom2`,
//! `EffectPrimitiveMotion.cpp:826-860`; dispatch
//! `EffectPrimitiveMotion.cpp:10084-10086` (`case PP_BOTTOM2:`).
//!
//! The four slots are vertical colour planes rooted at the master. Each live
//! slot spins `fullDisplayAngle` by 4 degrees per frame when `height[0] == 1`
//! (`kind == 1`) and by 2 otherwise, wraps the angle at 360, and derives its
//! height from the sine table:
//!
//! ```cpp
//! slot.maxHeight = 20.0f + 10.0f * g_sinTable[slot.fullDisplayAngle];
//! ```
//!
//! The renderer is
//! [`render_color_plane`](super::super::render_slots::render_color_plane)
//! (`EffectPrimitiveRendering.cpp:2710-2746`, dispatch `:744-745`), whose
//! `height[0]` switch picks the plane colour.
//!
//! The only body that spawns prim 48 is the two-argument
//! `CRagEffect::TexturedMagicRing(name, kind)` (`RagEffect.cpp:16466-16530`),
//! the ground ring of the bottom-skill family - see
//! [`super::super::rag::textured_magic_ring`]. (It is unrelated to
//! `EF_BOTTOM2` (137), the `PP_3DTEXTURE` cross of
//! [`super::super::rag::bottom_carpet`].)

use super::super::angles::Angles;
use super::{EffectPrimitiveState, MasterState};

/// One frame of `PrimBottom2`. The motion always returns `true`; the primitive
/// retires through the shared `stateCnt > duration` tail.
pub fn on_process(state: &mut EffectPrimitiveState, angles: &Angles, master: &MasterState) -> bool {
    // `this->render.position = master->render.position;`
    state.position = master.position;

    for index in 0..4usize {
        let slot = &mut state.slots[index];

        if slot.life != 1 {
            continue;
        }

        slot.process += 1;

        if slot.height[0] == 1.0 {
            slot.full_display_angle += 4;
        } else {
            slot.full_display_angle += 2;
        }

        if slot.full_display_angle >= 360 {
            slot.full_display_angle -= 360;
        }

        slot.max_height = 20.0 + 10.0 * angles.raw_sine(slot.full_display_angle.max(0) as usize);
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::{EffectPrimitiveSlot, PrimitiveId};

    fn state(kind: f32) -> EffectPrimitiveState {
        let mut state = EffectPrimitiveState {
            count: PrimitiveId::Bottom2,
            duration: 29999,
            ..Default::default()
        };

        for slot in &mut state.slots {
            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 0;
            slot.height[0] = kind;
            slot.vec_b_pre = [1.0, 0.0, -2.0];
            slot.vec_b_now = [-1.0, 0.0, 2.0];
        }

        state
    }

    #[test]
    fn primitive_id_is_48() {
        assert_eq!(PrimitiveId::Bottom2.id(), 48);
    }

    #[test]
    fn the_planes_follow_the_master_position() {
        let angles = Angles::new();
        let master = MasterState {
            position: [-3.0, 7.0, 2.0],
            ..Default::default()
        };
        let mut state = state(0.0);

        on_process(&mut state, &angles, &master);

        assert_eq!(state.position, [-3.0, 7.0, 2.0]);
    }

    #[test]
    fn kind_one_spins_twice_as_fast_as_the_other_kinds() {
        let angles = Angles::new();
        let mut fast = state(1.0);

        on_process(&mut fast, &angles, &MasterState::default());

        assert_eq!(fast.slots[0].process, 1);
        assert_eq!(fast.slots[0].full_display_angle, 4);

        let mut slow = state(0.0);
        on_process(&mut slow, &angles, &MasterState::default());
        assert_eq!(slow.slots[0].full_display_angle, 2);

        // Any other `height[0]` (including kind 2/3/4 and the untouched 0)
        // takes the slow branch.
        for kind in [2.0f32, 3.0, 4.0, 0.0] {
            let mut other = state(kind);
            on_process(&mut other, &angles, &MasterState::default());
            assert_eq!(other.slots[0].full_display_angle, 2, "kind {kind}");
        }
    }

    #[test]
    fn the_angle_wraps_at_three_hundred_sixty() {
        let angles = Angles::new();
        let mut fast = state(1.0);

        for slot in &mut fast.slots {
            slot.full_display_angle = 358;
        }

        on_process(&mut fast, &angles, &MasterState::default());

        // 358 + 4 = 362 -> 2.
        assert_eq!(fast.slots[0].full_display_angle, 2);

        let mut slow = state(0.0);
        for slot in &mut slow.slots {
            slot.full_display_angle = 359;
        }

        on_process(&mut slow, &angles, &MasterState::default());
        // 359 + 2 = 361 -> 1.
        assert_eq!(slow.slots[0].full_display_angle, 1);
    }

    #[test]
    fn max_height_is_the_offset_sine_of_the_angle() {
        let angles = Angles::new();
        let mut state = state(0.0);
        let mut expected_angle = 0i16;

        for _ in 0..200 {
            on_process(&mut state, &angles, &MasterState::default());
            expected_angle += 2;

            if expected_angle >= 360 {
                expected_angle -= 360;
            }

            assert_eq!(state.slots[0].full_display_angle, expected_angle);
            assert_eq!(
                state.slots[0].max_height,
                20.0 + 10.0 * angles.raw_sine(expected_angle as usize),
                "angle {expected_angle}"
            );
        }

        // The sine range keeps the height inside [10, 30].
        assert!((10.0..=30.0).contains(&state.slots[0].max_height));
    }

    #[test]
    fn dead_slots_are_untouched() {
        let angles = Angles::new();
        let mut state = state(0.0);

        for slot in &mut state.slots {
            slot.life = 0;
        }

        on_process(&mut state, &angles, &MasterState::default());

        for slot in &state.slots {
            assert_eq!(slot.process, 0);
            assert_eq!(slot.full_display_angle, 0);
            assert_eq!(slot.max_height, 0.0);
        }
    }
}
