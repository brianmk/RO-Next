//! `CRagEffect::TexturedMark(const char* name, char kind)` -
//! `RagEffect.cpp:4017-4060`.
//!
//! One prim 103 (`RenderStackedRings` / `PrimSpinFade`) on frame 0, textured
//! with `name` (`effect\storm2.tga` for every caller), `kind` selecting the
//! ring colour/blend and two variants:
//!
//! * `kind == 10` - the tall variant: 14 rings, `maxHeight = 14`, `height[0] =
//!   70`, no distance taper, `flag1[5] = 1` (fade starts after 120 frames),
//! * `kind == 2` - `flag1[5] = 10`, which never fades.
//!
//! The original copies `state.parent->render.position` into
//! `this->render.position` on every frame, before the `stateCnt != 0` early
//! return. The port's [`EffectRuntime::position`] *is* that anchor (the holder
//! sets it from the caster before the first step), so there is no second copy
//! to make; the primitive's `render.position` (set here because `PrimSpinFade`
//! does not call `FollowMaster`) is the frame-0 snapshot the original takes in
//! `CEffectPrim::Initialize` (`EffectPrimitiveState.cpp:79`).
//!
//! The one `ClampedRandom(360)` draw happens on frame 0 only. It is taken
//! before the primitive is launched so the runtime borrows stay separate; no
//! other draw happens in between, so the RNG order is unchanged.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// A ported `TexturedMark` call.
#[derive(Debug)]
pub struct TexturedMark {
    texture: &'static str,
    kind: i8,
    lifetime: i32,
}

impl TexturedMark {
    pub fn new(texture: &'static str, kind: i8, lifetime: i32) -> Self {
        Self { texture, kind, lifetime }
    }
}

impl PortedEffect for TexturedMark {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `this->render.position = this->state.parent->render.position;`
        // (represented by the runtime anchor, see the module docs)

        // `if (this->state.stateCnt != 0) return;`
        if runtime.frame != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let position = EffectRuntime::official_position(runtime.position);
        let kind = self.kind;

        // `static_cast<std::int16_t>(ClampedRandom(360))`
        let rot_start = runtime.rng.clamped_random(360) as i16;

        let mark = runtime.launch(PrimitiveId::StackedRings, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(mark);

        // `RSF_ALPHA | RSF_CUSTOM_BLEND`
        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.total_texture = 1;
        state.texture = Some(self.texture);
        state.size = 0.0;
        // `CEffectPrim::Initialize`'s `render.position = master.render.position`.
        state.position = position;

        state.slots[0].life = 0;
        state.slots[1].life = 0;
        state.slots[2].life = 0;
        state.slots[3].life = 0;

        state.slots[0].life = 1;
        state.slots[0].process = 0;
        state.slots[0].max_height = 40.0;
        state.slots[0].alpha_b = 0;
        state.slots[0].rot_start = rot_start;
        state.slots[0].distance = 2.0;
        state.slots[0].height[0] = 50.0;
        state.slots[0].height[1] = 5.0;
        state.slots[0].flag1[0] = 10;
        state.slots[0].flag1[4] = kind;
        state.slots[0].flag1[5] = 0;

        if kind == 10 {
            state.slots[0].max_height = 14.0;
            state.slots[0].height[0] = 70.0;
            state.slots[0].flag1[0] = 14;
            state.slots[0].distance = 0.0;
            state.slots[0].flag1[5] = 1;

            return true;
        }

        if kind == 2 {
            state.slots[0].flag1[5] = 10;
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::primitive::PrimitiveId;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    fn effect(kind: i8) -> TexturedMark {
        TexturedMark::new("effect\\storm2.tga", kind, 100)
    }

    #[test]
    fn spawns_one_stacked_rings_primitive() {
        let mut effect = effect(0);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let mark = &runtime.primitives[0].state;
        assert_eq!(mark.count, PrimitiveId::StackedRings);
        assert_eq!(mark.count.id(), 103);
        assert_eq!(mark.render_flag, 0x1 | 0x4);
        assert_eq!(mark.duration, 100);
        assert_eq!(mark.total_texture, 1);
        assert_eq!(mark.texture, Some("effect\\storm2.tga"));
        assert_eq!(mark.size, 0.0);
    }

    #[test]
    fn slot_zero_gets_the_official_defaults() {
        let mut effect = effect(0);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);

        // The mark's `ClampedRandom(360)` then PrimSpinFade's `rand() % 3`.
        let mut expected = crate::world::effect::port::rng::EffectRng::new(17);
        let rot_start = expected.clamped_random(360) as i16;
        let turn = expected.clamped_random(3) as i16;

        step(&mut effect, &mut runtime);

        let state = &runtime.primitives[0].state;

        for index in 1..4usize {
            assert_eq!(state.slots[index].life, 0);
        }

        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        // PrimSpinFade ran once after the spawn.
        assert_eq!(slot.process, 1);
        assert_eq!(slot.max_height, 40.0);
        assert_eq!(slot.alpha_b, 1);
        assert_eq!(slot.rot_start, (rot_start + turn + 3) % 360);
        assert_eq!(slot.distance, 2.0);
        assert_eq!(slot.height[0], 50.0);
        assert_eq!(slot.height[1], 5.0);
        assert_eq!(slot.flag1[0], 10);
        assert_eq!(slot.flag1[4], 0);
        assert_eq!(slot.flag1[5], 0);
    }

    #[test]
    fn the_primitive_keeps_the_effect_position() {
        let mut effect = effect(0);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);
        runtime.position = [10.0, 20.0, 30.0];

        step(&mut effect, &mut runtime);

        // Primitive state is official (Y-down): port y 20 -> -20.
        assert_eq!(runtime.primitives[0].state.position, [10.0, -20.0, 30.0]);
    }

    #[test]
    fn kind_ten_is_the_tall_fourteen_ring_variant() {
        let mut effect = effect(10);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);

        step(&mut effect, &mut runtime);

        let slot = &runtime.primitives[0].state.slots[0];
        assert_eq!(slot.max_height, 14.0);
        assert_eq!(slot.height[0], 70.0);
        assert_eq!(slot.flag1[0], 14);
        assert_eq!(slot.distance, 0.0);
        assert_eq!(slot.flag1[5], 1);
    }

    #[test]
    fn kind_two_never_fades() {
        let mut effect = effect(2);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);

        step(&mut effect, &mut runtime);

        let slot = &runtime.primitives[0].state.slots[0];
        assert_eq!(slot.max_height, 40.0);
        assert_eq!(slot.flag1[0], 10);
        assert_eq!(slot.flag1[5], 10);
    }

    #[test]
    fn kind_is_written_into_flag1_four() {
        for kind in [0i8, 1, 2, 3, 6, 8, 10, 11] {
            let mut effect = effect(kind);
            let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);

            step(&mut effect, &mut runtime);

            assert_eq!(runtime.primitives[0].state.slots[0].flag1[4], kind, "kind {kind}");
        }
    }

    #[test]
    fn the_mark_spawns_only_on_frame_zero() {
        let mut effect = effect(0);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.slots[0].process, 5);
    }

    #[test]
    fn the_live_mark_renders_the_stacked_rings() {
        use crate::world::effect::port::render_slots;

        let mut effect = effect(0);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 17);

        step(&mut effect, &mut runtime);

        // Ten rings, two triangles each; PrimSpinFade ramped alphaB to 1.
        assert_eq!(render_slots::triangles(&runtime.primitives[0].state).len(), 20);
    }
}
