//! `CRagEffect::QuadRingStack(const char* name)` - `RagEffect.cpp:14107-14183`.
//!
//! Dispatch: `case EF_DEFENDER:` (`RagEffect.cpp:24194-24196`) with
//! `effect\ring_black.tga`, and `case EF_REFLECTSHIELD:` (`:24197-24199`) with
//! `effect\ring_yellow.tga`. Lifetimes 200 for both (`EffectTypes.cpp:231`,
//! `:261`). Each id's dispatch is this body alone.
//!
//! The body arms one `PP_3DCASTING_2` (prim 24, `primitive/casting2.rs`) with
//! four concentric 360-degree rings - `maxHeight` 40/39/38/37, `distance`
//! 8.0/7.9/7.8/7.7, `rotStart` 180/270/0/90 - and zeroed `height`/`flag1`
//! arrays. The primitive's own motion spins and fades them; the shared
//! `Render3DCasting` renderer draws them.
//!
//! The original copies `state.parent->render.position` into `render.position`
//! on *every* frame, before the `stateCnt != 0` early return. The port's
//! [`EffectRuntime::position`] already is that anchor (the holder sets it from
//! the caster before the first step), so there is no second copy to make; the
//! primitive inherits it in `CEffectPrim::Initialize`
//! (`EffectPrimitiveState.cpp:79`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// A ported `QuadRingStack` call.
#[derive(Debug)]
pub struct QuadRingStack {
    texture: &'static str,
    lifetime: i32,
}

impl QuadRingStack {
    pub fn new(texture: &'static str, lifetime: i32) -> Self {
        Self { texture, lifetime }
    }
}

impl PortedEffect for QuadRingStack {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `CRagEffect::QuadRingStack` - `RagEffect.cpp:14107-14183`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `CopyVector(this->render.position, this->state.parent->render.position);`
        // happens before the gate; `runtime.position` is already that anchor.

        // `if (this->state.stateCnt != 0) return;`
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let position = runtime.position;

        let ring = runtime.launch(PrimitiveId::Casting2, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some(self.texture);
        state.size = 0.0;
        // `CEffectPrim::Initialize`'s `render.position = master.render.position`.
        state.position = position;

        const MAX_HEIGHTS: [f32; 4] = [40.0, 39.0, 38.0, 37.0];
        const ROT_STARTS: [i16; 4] = [180, 270, 0, 90];
        const DISTANCES: [f32; 4] = [8.0, 7.9, 7.8, 7.7];

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.full_display_angle = 360;
            slot.max_height = MAX_HEIGHTS[index];
            slot.rot_start = ROT_STARTS[index];
            slot.alpha_b = 0;
            slot.distance = DISTANCES[index];
            slot.rise_angle = 90;
            slot.vec_b_now[1] = 0.0;

            for height in 0..21usize {
                slot.height[height] = 0.0;
                slot.flag1[height] = 0;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn frame_zero_arms_four_stacked_rings() {
        let mut effect = QuadRingStack::new("effect\\ring_black.tga", 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 11);

        // Drive the body without the primitive pass so the configured slots
        // can be inspected before their motion touches them.
        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Casting2);
        assert_eq!(state.count.id(), 24);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.duration, 200);
        assert_eq!(state.texture, Some("effect\\ring_black.tga"));
        assert_eq!(state.size, 0.0);
        assert_eq!(state.position, runtime.position);

        const MAX_HEIGHTS: [f32; 4] = [40.0, 39.0, 38.0, 37.0];
        const ROT_STARTS: [i16; 4] = [180, 270, 0, 90];
        const DISTANCES: [f32; 4] = [8.0, 7.9, 7.8, 7.7];

        for index in 0..4usize {
            let slot = &state.slots[index];
            assert_eq!(slot.life, 1, "slot {index}");
            assert_eq!(slot.process, 0, "slot {index}");
            assert_eq!(slot.max_height, MAX_HEIGHTS[index], "slot {index}");
            assert_eq!(slot.rot_start, ROT_STARTS[index], "slot {index}");
            assert_eq!(slot.distance, DISTANCES[index], "slot {index}");
            assert_eq!(slot.rise_angle, 90, "slot {index}");
            assert_eq!(slot.alpha_b, 0, "slot {index}");
            assert_eq!(slot.full_display_angle, 360, "slot {index}");
            assert_eq!(slot.height, [0.0; 21], "slot {index}");
            assert_eq!(slot.flag1, [0; 21], "slot {index}");
        }
    }

    #[test]
    fn later_frames_do_not_spawn_again() {
        let mut effect = QuadRingStack::new("effect\\ring_yellow.tga", 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 5);

        step(&mut effect, &mut runtime);
        for _ in 0..10 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
        // The primitive's own motion has run on all eleven steps.
        assert_eq!(runtime.primitives[0].state.slots[0].process, 11);
        assert_eq!(runtime.primitives[0].state.slots[0].rot_start, 191);
        assert_eq!(runtime.primitives[0].state.slots[1].rot_start, 292);
        assert_eq!(runtime.primitives[0].state.slots[2].rot_start, 11);
        assert_eq!(runtime.primitives[0].state.slots[3].rot_start, 112);
    }

    #[test]
    fn the_ring_grows_its_alpha_then_fades_at_the_end_of_the_duration() {
        // The primitive's fade-out starts at `process == duration - 20`, so a
        // short duration exposes it quickly.
        let mut effect = QuadRingStack::new("effect\\ring_black.tga", 40);
        let mut runtime = EffectRuntime::new(40, [0.0, 0.0], 3);

        step(&mut effect, &mut runtime);
        // Frame 0 configured the slots and the primitive pass of the same step
        // advanced them once.
        assert_eq!(runtime.primitives[0].state.slots[0].process, 1);
        assert_eq!(runtime.primitives[0].state.slots[0].alpha_b, 1);

        for _ in 0..18 {
            step(&mut effect, &mut runtime);
        }

        // `process` 19 < 40 - 20 keeps the +1 ramp.
        assert_eq!(runtime.primitives[0].state.slots[0].process, 19);
        assert_eq!(runtime.primitives[0].state.slots[0].alpha_b, 19);

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives[0].state.slots[0].process, 20);
        assert_eq!(runtime.primitives[0].state.slots[0].alpha_b, 14);
    }
}
