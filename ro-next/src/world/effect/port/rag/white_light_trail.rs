//! `CRagEffect::WhiteLightTrail(const char* name, char kind)` -
//! `RagEffect.cpp:13451-13516`.
//!
//! One prim 96 (`PP_PEONGUP`, `rag/../primitive/peong_up.rs`) on exactly one
//! frame: frame 0 for `kind == 1`, frame 35 for `kind == 0`. The primitive
//! carries the caller's texture (the Storm Kick bodies pass
//! `effect\whitelight.tga`) and arms either one slot (`kind == 0`) or all four
//! (`kind == 1`).
//!
//! `kind == 0` is the single-slot variant used by `EF_PEONG`
//! (`RagEffect.cpp:24297`); `kind == 1` is the four-slot variant used by
//! `EF_MOVE_COORDINATE` (`:22224`) and both Storm Kick bodies (`:23709`,
//! `:23720`). Only these two kinds are defined: the original declares
//! `std::int16_t limit` without initialising it for any other `kind`, and
//! comparing against that indeterminate value is undefined behaviour, so the
//! port implements exactly the two branches the bodies use.
//!
//! RNG draw order is part of the body: `maxHeight`, `distance`, `process`,
//! `vecBPre.y`, the heading, then `height[0..4]` (the `kind == 0` branch draws
//! the same nine values once, the `kind == 1` branch nine per slot).

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// A ported `WhiteLightTrail` call. It is a helper body, not a registered
/// effect id: the owning `CRagEffect` case calls it once per instance and the
/// body's own frame gate limits it to one emission.
#[derive(Debug)]
pub struct WhiteLightTrail {
    texture: &'static str,
    kind: i8,
    lifetime: i32,
}

impl WhiteLightTrail {
    pub fn new(texture: &'static str, kind: i8, lifetime: i32) -> Self {
        Self { texture, kind, lifetime }
    }
}

impl PortedEffect for WhiteLightTrail {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `std::int16_t limit; if (kind == 0) limit = 35; else if (kind == 1)
        // limit = 0;` - see the module note about the other kinds.
        let limit = if self.kind == 0 { 35 } else { 0 };

        if runtime.frame != limit {
            return true;
        }

        let angles = Angles::shared();
        let lifetime = runtime.lifetime;
        let kind = self.kind;

        // Draw every `ClampedRandom` before borrowing the launched primitive:
        // the original's draw order is maxHeight, distance, process,
        // `vecBPre.y`, the heading and the four `height[0..4]` values, once for
        // kind 0 and once per slot for kind 1. Keeping the draws in locals does
        // not change that order.
        let mut slots = [SlotSetup::default(); 4];

        if kind == 0 {
            slots[0] = draw_slot(runtime, &angles, 0);
        } else if kind == 1 {
            for slot in &mut slots {
                *slot = draw_slot(runtime, &angles, 1);
            }
        }

        let index = runtime.launch(PrimitiveId::PeongUp, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
        state.render_flag = 1 | 4;
        // `state.duration = this->state.duration;`
        state.duration = lifetime;
        // `state.totalTexture` keeps the constructor's 1; the body allocates the
        // texture array and loads `name` into slot 0.
        state.texture = Some(self.texture);
        // `state.size = 0.0f;`
        state.size = 0.0;

        for slot in &mut state.slots {
            slot.life = 0;
        }

        if kind == 0 {
            // `int slot0 = 0;` - the single-slot branch arms slot zero only.
            state.slots[0].life = 1;
            state.slots[0].max_height = slots[0].max_height;
            state.slots[0].distance = slots[0].distance;
            state.slots[0].process = slots[0].process;
            state.slots[0].alpha_b = 0;
            state.slots[0].vec_b_pre = slots[0].vec_b_pre;
            state.slots[0].height[0..4].copy_from_slice(&slots[0].height);
            state.slots[0].flag1[0] = 1;

            return true;
        }

        if kind == 1 {
            for index in 0..4usize {
                let setup = slots[index];
                let slot = &mut state.slots[index];

                slot.life = 1;
                slot.max_height = setup.max_height;
                slot.distance = setup.distance;
                slot.process = setup.process;
                slot.alpha_b = 0;
                slot.vec_b_pre = setup.vec_b_pre;
                slot.height[0..4].copy_from_slice(&setup.height);
                slot.flag1[0] = 2;
            }
        }

        true
    }
}

/// The nine per-slot random values of one `WhiteLightTrail` slot.
#[derive(Default, Clone, Copy)]
struct SlotSetup {
    max_height: f32,
    distance: f32,
    process: i32,
    vec_b_pre: [f32; 3],
    height: [f32; 4],
}

/// Draws one slot's values in the exact order the decompilation does.
///
/// `kind == 0` uses `ClampedRandom(16)`/`(11)`/`(26)`/`(8)`/`(360)` and the
/// 2.5-unit heading radius; `kind == 1` uses `ClampedRandom(11)` for both
/// `maxHeight` and `distance`, `* 0.005 + 0.08` / `* 0.05 + 2.0`, and the
/// 3.5-unit heading radius.
fn draw_slot(runtime: &mut EffectRuntime, angles: &Angles, kind: i8) -> SlotSetup {
    let mut setup = SlotSetup::default();

    if kind == 0 {
        setup.max_height = (runtime.rng.clamped_random(16) as f32) * 0.01 + 0.1;
        setup.distance = (runtime.rng.clamped_random(11) as f32) * 0.1 + 3.0;
        setup.process = -runtime.rng.clamped_random(26);
        setup.vec_b_pre[1] = -(runtime.rng.clamped_random(8) as f32);

        let heading = runtime.rng.clamped_random(360) as usize;
        setup.vec_b_pre[0] = angles.raw_cosine(heading) * 2.5;
        setup.vec_b_pre[2] = angles.raw_sine(heading) * 2.5;

        for height in &mut setup.height {
            *height = runtime.rng.clamped_random(360) as f32;
        }

        return setup;
    }

    setup.max_height = (runtime.rng.clamped_random(11) as f32) * 0.005 + 0.08;
    setup.distance = (runtime.rng.clamped_random(11) as f32) * 0.05 + 2.0;
    setup.process = -runtime.rng.clamped_random(26);
    setup.vec_b_pre[1] = -(runtime.rng.clamped_random(8) as f32);

    let heading = runtime.rng.clamped_random(360) as usize;
    setup.vec_b_pre[0] = angles.raw_cosine(heading) * 3.5;
    setup.vec_b_pre[2] = angles.raw_sine(heading) * 3.5;

    for height in &mut setup.height {
        *height = runtime.rng.clamped_random(360) as f32;
    }

    setup
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::EffectRuntime;

    fn close(left: f32, right: f32) -> bool {
        (left - right).abs() < 1e-5
    }

    #[test]
    fn kind_one_spawns_one_primitive_on_frame_zero_only() {
        let mut effect = WhiteLightTrail::new("effect\\whitelight.tga", 1, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 17);

        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.count, PrimitiveId::PeongUp);
        assert_eq!(runtime.primitives[0].state.count.id(), 96);

        runtime.frame = 1;
        effect.on_process(&mut runtime);
        assert_eq!(runtime.primitives.len(), 1, "later frames do not spawn");
    }

    #[test]
    fn kind_one_arms_all_four_slots_with_the_official_ranges() {
        let mut effect = WhiteLightTrail::new("effect\\whitelight.tga", 1, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 17);

        effect.on_process(&mut runtime);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.duration, 200);
        assert_eq!(state.texture, Some("effect\\whitelight.tga"));
        assert_eq!(state.total_texture, 1);
        assert_eq!(state.size, 0.0);
        // `CEffectPrim::Initialize` anchors the primitive at the master.
        assert_eq!(state.position, runtime.position);

        for slot in &state.slots {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.alpha_b, 0);
            assert_eq!(slot.flag1[0], 2);
            assert!((0.08..0.135).contains(&slot.max_height));
            assert!((2.0..=2.5).contains(&slot.distance));
            assert!((-25..=0).contains(&slot.process));
            assert!((-7.0..=0.0).contains(&slot.vec_b_pre[1]));
            // The heading offsets lie on the 3.5 radius circle.
            let radius = (slot.vec_b_pre[0] * slot.vec_b_pre[0] + slot.vec_b_pre[2] * slot.vec_b_pre[2]).sqrt();
            assert!(close(radius, 3.5), "radius was {radius}");
            for height in &slot.height[0..4] {
                assert!((0.0..360.0).contains(height));
            }
        }
    }

    #[test]
    fn kind_zero_waits_until_frame_thirty_five_and_arms_only_slot_zero() {
        let mut effect = WhiteLightTrail::new("effect\\whitelight.tga", 0, 100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 5);

        for frame in 0..35 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        assert!(runtime.primitives.is_empty(), "kind 0 spawns on frame 35 only");

        runtime.frame = 35;
        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.slots[0].life, 1);
        assert_eq!(state.slots[0].flag1[0], 1);
        assert!((0.1..0.26).contains(&state.slots[0].max_height));
        assert!((3.0..4.1).contains(&state.slots[0].distance));
        // The 2.5 radius heading circle of the kind-0 branch.
        let slot = &state.slots[0];
        let radius = (slot.vec_b_pre[0] * slot.vec_b_pre[0] + slot.vec_b_pre[2] * slot.vec_b_pre[2]).sqrt();
        assert!(close(radius, 2.5), "radius was {radius}");

        for index in 1..4 {
            assert_eq!(state.slots[index].life, 0);
        }
    }

    #[test]
    fn the_rng_draw_order_matches_the_decompilation() {
        // Reproduce the exact sequence of draws by hand: one primitive launch
        // on frame 0, then the 36 `ClampedRandom` values of the four slots.
        let mut expected = crate::world::effect::port::rng::EffectRng::new(9);
        let mut effect = WhiteLightTrail::new("effect\\whitelight.tga", 1, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 9);

        effect.on_process(&mut runtime);

        let head = &runtime.primitives[0].state;
        let angles = Angles::shared();

        for index in 0..4 {
            let max_height = (expected.clamped_random(11) as f32) * 0.005 + 0.08;
            let distance = (expected.clamped_random(11) as f32) * 0.05 + 2.0;
            let process = -expected.clamped_random(26);
            let y = -(expected.clamped_random(8) as f32);
            let heading = expected.clamped_random(360) as usize;
            let height = [
                expected.clamped_random(360) as f32,
                expected.clamped_random(360) as f32,
                expected.clamped_random(360) as f32,
                expected.clamped_random(360) as f32,
            ];

            let slot = head.slots[index];
            assert_eq!(slot.max_height, max_height, "slot {index}");
            assert_eq!(slot.distance, distance, "slot {index}");
            assert_eq!(slot.process, process, "slot {index}");
            assert_eq!(slot.vec_b_pre[1], y, "slot {index}");
            assert_eq!(slot.vec_b_pre[0], angles.raw_cosine(heading) * 3.5, "slot {index}");
            assert_eq!(slot.vec_b_pre[2], angles.raw_sine(heading) * 3.5, "slot {index}");
            assert_eq!(&slot.height[0..4], &height, "slot {index}");
        }
    }
}
