//! `CRagEffect::TexturedGlow` - `RagEffect.cpp:5516-5565`.
//!
//! The spinning glow columns of `EF_BOTTOM_ETERNALCHAOS` (280, size 8) and
//! `EF_BOTTOM_SIEGFRIED` (285, size 4), dispatched at
//! `RagEffect.cpp:24006-24007` and `:24021-24022`.
//!
//! The body copies the parent anchor, gates on frame 0 and arms **prim 50**,
//! whose id has no `EffectPrimId.h` symbol (the enum jumps from 49 to 52). Only
//! slot 0 is live; the other three are explicitly zeroed by the original before
//! the slots are armed:
//!
//! * `size` becomes `state.size` (the renderer's `size` colour/alpha table),
//! * `maxHeight = ClampedRandom(10) + 25`, `rotStart = ClampedRandom(360)`,
//!   `alphaB = 50 - ClampedRandom(10)`, `alphaT = ClampedRandom(360)`,
//! * `fullDisplayAngle = 315`, `riseAngle = 90`, `process = 0`,
//! * the 21 `height[]`/`flag1[]` entries are then zeroed.
//!
//! RNG order per spawn: `maxHeight`, `rotStart`, `alphaB`, `alphaT`.
//!
//! Lifetimes: `EF_BOTTOM_ETERNALCHAOS` 29999 (`EffectTypes.cpp:289`),
//! `EF_BOTTOM_SIEGFRIED` 29999 (`:294`). Renderer: the shared
//! [`render_3d_casting`](super::super::render_slots::render_3d_casting)
//! (`Render3DCasting`, `EffectPrimitiveRendering.cpp:6596-6675`, dispatch
//! `:504`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// The `TexturedGlow` dispatch: one `TexturedGlow(name, size)` call.
#[derive(Debug)]
pub struct TexturedGlow {
    texture: &'static str,
    size: i8,
    lifetime: i32,
}

impl TexturedGlow {
    pub const fn new(texture: &'static str, size: i8, lifetime: i32) -> Self {
        Self { texture, size, lifetime }
    }
}

/// `CRagEffect::TexturedGlow(const char* name, char size)` -
/// `RagEffect.cpp:5516-5565`.
fn textured_glow(runtime: &mut EffectRuntime, name: &'static str, size: i8) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    let lifetime = runtime.lifetime;

    // `this->state.lastPrim = LaunchEffectPrim(50, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::TexturedGlow, [0.0, 0.0, 0.0]);

    // RNG order: maxHeight (10), rotStart (360), alphaB (10), alphaT (360).
    // Taken before the state borrow: the effect and its primitives share the
    // RNG.
    let max_height = runtime.rng.clamped_random(10) as f32 + 25.0;
    let rot_start = runtime.rng.clamped_random(360) as i16;
    let alpha_b = 50 - runtime.rng.clamped_random(10) as i16;
    let alpha_t = runtime.rng.clamped_random(360) as i16;

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some(name);
    state.size = size as f32;

    for slot in &mut state.slots {
        slot.life = 0;
    }

    let slot = &mut state.slots[0];

    slot.life = 1;
    slot.process = 0;
    slot.full_display_angle = 315;
    slot.max_height = max_height;
    slot.rot_start = rot_start;
    slot.alpha_b = alpha_b;
    slot.rise_angle = 90;
    slot.alpha_t = alpha_t;
    slot.vec_b_now[1] = 0.0;

    for index in 0..21usize {
        slot.height[index] = 0.0;
        slot.flag1[index] = 0;
    }
}

impl PortedEffect for TexturedGlow {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `case EF_BOTTOM_ETERNALCHAOS:` / `case EF_BOTTOM_SIEGFRIED:` -
    /// `RagEffect.cpp:24006-24007`, `:24021-24022`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        textured_glow(runtime, self.texture, self.size);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::rng::EffectRng;
    use crate::world::effect::port::runtime::step;

    #[test]
    fn eternal_chaos_spawns_the_size_eight_glow() {
        let mut effect = TexturedGlow::new("effect\\twirl.bmp", 8, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 7);

        // Predict the four RNG draws in order.
        let mut expected = EffectRng::new(7);
        let max_height = expected.clamped_random(10) as f32 + 25.0;
        let rot_start = expected.clamped_random(360) as i16;
        let alpha_b = 50 - expected.clamped_random(10) as i16;
        let alpha_t = expected.clamped_random(360) as i16;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::TexturedGlow);
        assert_eq!(state.texture, Some("effect\\twirl.bmp"));
        assert_eq!(state.size, 8.0);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.duration, 29999);

        // The inline motion runs in the same step: process 1, alphaT +1.
        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        assert_eq!(slot.process, 1);
        assert_eq!(slot.full_display_angle, 315);
        assert_eq!(slot.rise_angle, 90);
        assert_eq!(slot.max_height, max_height);
        assert_eq!(slot.rot_start, (rot_start + 3) % 360);
        assert_eq!(slot.alpha_b, alpha_b);
        assert_eq!(slot.alpha_t, (alpha_t + 1) % 360);
        assert_eq!(
            slot.distance,
            crate::world::effect::port::angles::Angles::shared().raw_sine(slot.alpha_t as usize) + 3.0
        );

        for slot in &state.slots[1..] {
            assert_eq!(slot.life, 0);
        }
    }

    #[test]
    fn siegfried_uses_size_four() {
        let mut effect = TexturedGlow::new("effect\\twirl.bmp", 4, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 3);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[0].state.size, 4.0);
    }

    #[test]
    fn later_frames_spawn_nothing() {
        let mut effect = TexturedGlow::new("effect\\twirl.bmp", 8, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 3);

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
    }

    #[test]
    fn the_anchor_follows_the_parent() {
        let mut effect = TexturedGlow::new("effect\\twirl.bmp", 8, 29999);
        let mut runtime = EffectRuntime::new(29999, [0.0, 0.0], 3);
        runtime.parent_position = Some([10.0, 20.0, 30.0]);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.position, [10.0, 20.0, 30.0]);
        // Primitive state is official (Y-down): port y 20 -> -20.
        assert_eq!(runtime.primitives[0].state.position, [10.0, -20.0, 30.0]);
    }
}
