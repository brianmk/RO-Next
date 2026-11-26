//! `CRagEffect::TexturedMagicRing(const char* name, char kind)` -
//! `RagEffect.cpp:16466-16530`.
//!
//! This is the **two-argument** overload (the `char type` one at `:15565` is
//! [`super::magic_ring`]). It arms one `PP_BOTTOM2` (prim 48,
//! `primitive/bottom2.rs`) with four vertical colour planes and the caller's
//! texture, and the shared `RenderColorPlane` renderer draws them: the plane
//! colour and blend come from `slot.height[0]`, which the body sets to `kind`
//! for every `kind` except 0.
//!
//! Five ids dispatch to it, each with this body alone, all lifetime 29999
//! (`EffectTypes.cpp:286`, `:296`, `:299`, `:301`, `:303`):
//!
//! | id | effect | name | kind |
//! | --- | --- | --- | --- |
//! | 277 | `EF_BOTTOM_DISSONANCE` | `effect\ring_blue.tga` | 0 |
//! | 287 | `EF_BOTTOM_ASSASSINCROSS` | `effect\ring_red.tga` | 1 |
//! | 290 | `EF_BOTTOM_UGLYDANCE` | `effect\ring_red.tga` | 0 |
//! | 292 | `EF_BOTTOM_DONTFORGETME` | `effect\magic_green.tga` | 2 |
//! | 294 | `EF_BOTTOM_SERVICEFORYOU` | `effect\safeline.bmp` | 4 |
//!
//! RNG order is part of the body: one `ClampedRandom(360)` for
//! `fullDisplayAngle` per slot, then the offset draws of the kind - two
//! `ClampedRandom(40)` for kind 0, one `ClampedRandom(7)` for kind 1/2 (the
//! x draw for slots 0/1, the z draw for slots 2/3) and none for kind 3/4. The
//! draws happen for all four slots, including the two dead slots of kind 3/4.
//!
//! The original copies `state.parent->render.position` into `render.position`
//! first; the port's [`EffectRuntime::position`] is already that anchor.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// A ported `TexturedMagicRing(name, kind)` call.
#[derive(Debug)]
pub struct TexturedMagicRing {
    texture: &'static str,
    kind: i8,
    lifetime: i32,
}

impl TexturedMagicRing {
    pub fn new(texture: &'static str, kind: i8, lifetime: i32) -> Self {
        Self { texture, kind, lifetime }
    }
}

/// The random values one slot consumes, drawn in the original's order.
#[derive(Default, Clone, Copy)]
struct SlotRandom {
    angle: i16,
    x: f32,
    z: f32,
}

/// Draws one slot's values. `index` selects which of the two kind-1/2 offset
/// draws happens; `kind` selects how many draws there are.
fn draw_slot(runtime: &mut EffectRuntime, kind: i8, index: usize) -> SlotRandom {
    let mut random = SlotRandom {
        // `static_cast<std::int16_t>(ClampedRandom(360))`
        angle: runtime.rng.clamped_random(360) as i16,
        ..Default::default()
    };

    if kind == 0 {
        // `static_cast<float>(ClampedRandom(40) - 20)`
        random.x = (runtime.rng.clamped_random(40) - 20) as f32;
        random.z = (runtime.rng.clamped_random(40) - 20) as f32;
    } else if kind == 1 || kind == 2 {
        if index < 2 {
            random.x = runtime.rng.clamped_random(7) as f32 - 3.0;
        } else {
            random.z = runtime.rng.clamped_random(7) as f32 - 3.0;
        }
    }

    random
}

impl PortedEffect for TexturedMagicRing {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `CRagEffect::TexturedMagicRing` - `RagEffect.cpp:16466-16530`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `CopyVector(this->render.position, this->state.parent->render.position);`
        // (represented by the runtime anchor, see the module docs)

        // `if (this->state.stateCnt != 0) return;`
        if runtime.frame != 0 {
            return true;
        }

        let kind = self.kind;
        let lifetime = runtime.lifetime;

        // Every slot's draws happen before the primitive is touched, in slot
        // order; the number of draws depends only on `kind`/`index`.
        let mut randoms = [SlotRandom::default(); 4];
        for (index, random) in randoms.iter_mut().enumerate() {
            *random = draw_slot(runtime, kind, index);
        }

        let ring = runtime.launch(PrimitiveId::Bottom2, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        // `state.renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND;`
        state.render_flag = 1 | 4;
        state.duration = lifetime;
        state.texture = Some(self.texture);
        state.size = 0.0;

        for (index, (slot, random)) in state.slots.iter_mut().zip(randoms).enumerate() {
            if kind == 0 {
                slot.life = 1;
                slot.process = 0;
                slot.full_display_angle = random.angle;
                slot.vec_b_pre[0] = random.x;
                slot.vec_b_pre[2] = random.z;
                slot.vec_b_now[0] = 0.0;
                slot.vec_b_now[2] = 0.0;
                slot.alpha_b = 15;
            } else if kind == 1 || kind == 2 {
                slot.life = 1;
                slot.process = 0;
                slot.full_display_angle = random.angle;

                if index < 2 {
                    slot.vec_b_pre[0] = random.x;
                    slot.vec_b_pre[2] = -3.0;
                    slot.vec_b_now[0] = -slot.vec_b_pre[0];
                    slot.vec_b_now[2] = 3.0;
                } else {
                    slot.vec_b_pre[0] = -3.0;
                    slot.vec_b_pre[2] = random.z;
                    slot.vec_b_now[0] = 3.0;
                    slot.vec_b_now[2] = -slot.vec_b_pre[2];
                }

                slot.alpha_b = 50;
                slot.height[0] = f32::from(kind);
            } else if kind == 3 || kind == 4 {
                // `life` is the only value that differs for slots 2/3; every
                // other write (and the RNG draw) still happens.
                slot.life = if index < 2 { 1 } else { 0 };
                slot.process = 0;
                slot.full_display_angle = random.angle;

                if index < 1 {
                    slot.vec_b_pre[0] = 6.0;
                    slot.vec_b_pre[2] = 0.0;
                    slot.vec_b_now[0] = -6.0;
                    slot.vec_b_now[2] = 0.0;
                } else {
                    slot.vec_b_pre[0] = 0.0;
                    slot.vec_b_pre[2] = -6.0;
                    slot.vec_b_now[0] = 0.0;
                    slot.vec_b_now[2] = 6.0;
                }

                slot.alpha_b = 100;
                slot.height[0] = f32::from(kind);
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::rng::EffectRng;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn kind_zero_scatters_four_planes_with_random_offsets() {
        let mut effect = TexturedMagicRing::new("effect\\ring_blue.tga", 0, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 21);

        // Drive the body without the primitive pass: the assertions below are
        // about the values the body writes, before `PrimBottom2` spins them.
        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Bottom2);
        assert_eq!(state.count.id(), 48);
        assert_eq!(state.render_flag, 1 | 4);
        assert_eq!(state.duration, 29999);
        assert_eq!(state.texture, Some("effect\\ring_blue.tga"));
        assert_eq!(state.size, 0.0);

        for slot in &state.slots {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.process, 0);
            assert_eq!(slot.alpha_b, 15);
            assert_eq!(slot.height[0], 0.0);
            assert!((0..360).contains(&slot.full_display_angle));
            // The movement starts at the origin; only `vecBPre` is scattered.
            assert_eq!(slot.vec_b_now[0], 0.0);
            assert_eq!(slot.vec_b_now[2], 0.0);
            assert!((-20.0..=19.0).contains(&slot.vec_b_pre[0]));
            assert!((-20.0..=19.0).contains(&slot.vec_b_pre[2]));
        }

        // `PrimBottom2` has not run yet: the body leaves `process` at zero.
        assert_eq!(state.slots[0].process, 0);
    }

    #[test]
    fn kind_one_and_two_use_the_two_by_two_offsets() {
        for kind in [1i8, 2] {
            let mut effect = TexturedMagicRing::new("effect\\ring_red.tga", kind, 29999);
            let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 3);

            step(&mut effect, &mut runtime);

            let state = &runtime.primitives[0].state;
            assert_eq!(state.texture, Some("effect\\ring_red.tga"));

            for (index, slot) in state.slots.iter().enumerate() {
                assert_eq!(slot.life, 1);
                assert_eq!(slot.alpha_b, 50);
                assert_eq!(slot.height[0], f32::from(kind));

                if index < 2 {
                    assert_eq!(slot.vec_b_pre[2], -3.0);
                    assert_eq!(slot.vec_b_now[0], -slot.vec_b_pre[0]);
                    assert_eq!(slot.vec_b_now[2], 3.0);
                    assert!((-3.0..=3.0).contains(&slot.vec_b_pre[0]));
                } else {
                    assert_eq!(slot.vec_b_pre[0], -3.0);
                    assert_eq!(slot.vec_b_now[0], 3.0);
                    assert_eq!(slot.vec_b_now[2], -slot.vec_b_pre[2]);
                    assert!((-3.0..=3.0).contains(&slot.vec_b_pre[2]));
                }
            }
        }
    }

    #[test]
    fn kind_three_and_four_keep_only_two_live_planes() {
        for kind in [3i8, 4] {
            let mut effect = TexturedMagicRing::new("effect\\safeline.bmp", kind, 29999);
            let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 9);

            step(&mut effect, &mut runtime);

            let state = &runtime.primitives[0].state;
            assert_eq!(state.texture, Some("effect\\safeline.bmp"));

            assert_eq!(state.slots[0].life, 1);
            assert_eq!(state.slots[1].life, 1);
            assert_eq!(state.slots[2].life, 0);
            assert_eq!(state.slots[3].life, 0);

            // The cross: slot 0 along x, slot 1 along z.
            assert_eq!(state.slots[0].vec_b_pre, [6.0, 0.0, 0.0]);
            assert_eq!(state.slots[0].vec_b_now, [-6.0, 0.0, 0.0]);
            assert_eq!(state.slots[1].vec_b_pre, [0.0, 0.0, -6.0]);
            assert_eq!(state.slots[1].vec_b_now, [0.0, 0.0, 6.0]);

            // The dead slots still receive every write (only `life` differs).
            for slot in &state.slots[2..] {
                assert_eq!(slot.alpha_b, 100);
                assert_eq!(slot.height[0], f32::from(kind));
                assert_eq!(slot.vec_b_pre, [0.0, 0.0, -6.0]);
                assert_eq!(slot.vec_b_now, [0.0, 0.0, 6.0]);
            }
        }
    }

    #[test]
    fn later_frames_do_not_launch_a_second_ring() {
        let mut effect = TexturedMagicRing::new("effect\\ring_red.tga", 0, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 2);

        for _ in 0..30 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.primitives[0].state.slots[0].process, 30);
    }

    #[test]
    fn the_rng_draw_order_matches_the_decompilation() {
        for kind in [0i8, 1, 3] {
            let mut expected = EffectRng::new(17);
            let mut effect = TexturedMagicRing::new("effect\\ring_blue.tga", kind, 29999);
            let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 17);

            // No primitive pass: `PrimBottom2` would add to the angle.
            effect.on_process(&mut runtime);

            let state = &runtime.primitives[0].state;

            for index in 0..4usize {
                let angle = expected.clamped_random(360) as i16;
                let (x, z) = if kind == 0 {
                    (
                        (expected.clamped_random(40) - 20) as f32,
                        (expected.clamped_random(40) - 20) as f32,
                    )
                } else if kind == 1 || kind == 2 {
                    if index < 2 {
                        (expected.clamped_random(7) as f32 - 3.0, 0.0)
                    } else {
                        (0.0, expected.clamped_random(7) as f32 - 3.0)
                    }
                } else {
                    (0.0, 0.0)
                };

                let slot = &state.slots[index];
                assert_eq!(slot.full_display_angle, angle, "kind {kind} slot {index}");
                if kind == 0 {
                    assert_eq!(slot.vec_b_pre[0], x, "kind {kind} slot {index}");
                    assert_eq!(slot.vec_b_pre[2], z, "kind {kind} slot {index}");
                } else if kind == 1 || kind == 2 {
                    if index < 2 {
                        assert_eq!(slot.vec_b_pre[0], x, "kind {kind} slot {index}");
                    } else {
                        assert_eq!(slot.vec_b_pre[2], z, "kind {kind} slot {index}");
                    }
                }
            }
        }
    }
}
