//! `CRagEffect::JobLvUp50_2(short angle)` - `RagEffect.cpp:2568-2589`.
//!
//! Dispatch: `case EF_KAIZEL:` (590) - `RagEffect.cpp:21287-21292`:
//!
//! ```cpp
//! case EF_KAIZEL:
//!   if (this->state.stateCnt == 15) {
//!     this->JobLvUp50_2(0);
//!     this->JobLvUp50_2(45);
//!   }
//!   break;
//! ```
//!
//! Lifetime 200 (`EffectTypes.cpp:599`). The dispatch is the body alone (no
//! wave, no `.str` clip), so `EF_KAIZEL` is registered with [`Kaizel`].
//!
//! `JobLvUp50_2` has **no** internal `stateCnt` gate - the caller gates it - so
//! each call launches a fresh `PP_SLASH1` (prim 75, `primitive/slash1.rs`)
//! with `effect\magic_blue.tga`, `size = 4`, and four blades spaced 90 degrees
//! apart from `angle`, `maxHeight = 1.5`, `distance = 2`, `height[0] = 1` and
//! `flag1[0] = 0` (the slow growth variant). The renderer is the shared
//! [`render_slash_1`](super::super::render_slots::render_slash_1).
//!
//! The original copies `state.parent->render.position` into `render.position`
//! first; the port's [`EffectRuntime::position`] is already that anchor.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// The `EF_KAIZEL` body: two `JobLvUp50_2` calls on frame 15.
#[derive(Debug)]
pub struct Kaizel {
    lifetime: i32,
}

impl Kaizel {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }

    /// `CRagEffect::JobLvUp50_2` - `RagEffect.cpp:2568-2589`.
    fn job_lv_up_50_2(runtime: &mut EffectRuntime, angle: i16) {
        // `CopyVector(this->render.position, this->state.parent->render.position);`
        // (represented by the runtime anchor, see the module docs)

        let lifetime = runtime.lifetime;

        let slash = runtime.launch(PrimitiveId::Slash1, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(slash);

        // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some("effect\\magic_blue.tga");
        state.size = 4.0;

        for index in 0..4i16 {
            let slot = &mut state.slots[index as usize];

            slot.life = 1;
            slot.process = 0;
            slot.max_height = 1.5;
            slot.rot_start = index * 90 + angle;
            slot.alpha_b = 0;
            slot.distance = 2.0;
            slot.height[0] = 1.0;
            slot.flag1[0] = 0;
        }
    }
}

impl PortedEffect for Kaizel {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `case EF_KAIZEL:` - `RagEffect.cpp:21287-21292`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 15 {
            Self::job_lv_up_50_2(runtime, 0);
            Self::job_lv_up_50_2(runtime, 45);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn nothing_happens_before_frame_fifteen() {
        let mut effect = Kaizel::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 7);

        for _ in 0..15 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.frame, 15);
        assert!(runtime.primitives.is_empty());
    }

    #[test]
    fn frame_fifteen_launches_two_slash_rings() {
        let mut effect = Kaizel::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 7);

        for _ in 0..16 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2);

        for (index, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            let angle = if index == 0 { 0 } else { 45 };

            assert_eq!(state.count, PrimitiveId::Slash1);
            assert_eq!(state.count.id(), 75);
            assert_eq!(state.render_flag, 1 | 4);
            assert_eq!(state.duration, 200);
            assert_eq!(state.texture, Some("effect\\magic_blue.tga"));
            assert_eq!(state.size, 4.0);

            for (slot_index, slot) in state.slots.iter().enumerate() {
                assert_eq!(slot.life, 1);
                assert_eq!(slot.process, 1, "the same step advanced the blades");
                // `flag1[0] = 0`: distance +2, maxHeight +0.4 (cap 4), alpha +5.
                assert_eq!(slot.max_height, 1.9);
                assert_eq!(slot.rot_start, slot_index as i16 * 90 + angle);
                assert_eq!(slot.distance, 4.0);
                assert_eq!(slot.height[0], 2.0);
                assert_eq!(slot.alpha_b, 5);
                assert_eq!(slot.flag1[0], 0);
            }
        }

        // `rotStart` of the second call is the first call's plus 45.
        assert_eq!(runtime.primitives[1].state.slots[0].rot_start, 45);
        assert_eq!(runtime.primitives[1].state.slots[3].rot_start, 45 + 270);
    }

    #[test]
    fn only_frame_fifteen_launches() {
        let mut effect = Kaizel::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 7);

        for _ in 0..40 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2);
    }
}
