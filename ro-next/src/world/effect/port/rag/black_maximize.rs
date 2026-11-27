//! `EF_GUARD2` (496) - `RagEffect.cpp:21646-21658`.
//!
//! The black-maximize shell: the dispatch calls
//! `CRagEffect::BlackMaximize("effect\\a01.bmp")` (`RagEffect.cpp:3453-3480`,
//! prim 61 `PP_GUARD`), then on frames 10..30 alternates the caster colour
//! between `(250,250,250)` on even frames and `(255,255,255)` on odd frames,
//! requesting the additive body flag (`render.flags & 1`) on the even frames.
//!
//! The body plays `effect\\black_maximize_power_sword_bic.wav` and spawns the
//! cage on frame 0 only. The renderer is `RenderSphereCage`
//! (`EffectPrimitiveRendering.cpp:4328`); the motion is `PrimGuard`
//! (`EffectPrimitiveMotion.cpp:1344-1383`).
//!
//! Lifetime: `EF_GUARD2` 200 (`EffectTypes.cpp:505`).

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `case EF_GUARD2:` dispatch - `RagEffect.cpp:21646-21658`.
#[derive(Debug)]
pub struct Guard2 {
    lifetime: i32,
}

impl Guard2 {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

/// `CRagEffect::BlackMaximize(const char* name)` - `RagEffect.cpp:3453-3480`.
fn black_maximize(runtime: &mut EffectRuntime, name: &'static str) {
    // `const RenderObjectState& parent = this->state.parent->render;`
    // `this->render.position = parent.position;`
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    runtime.play_sound("effect\\black_maximize_power_sword_bic.wav");

    let lifetime = runtime.lifetime;

    // `this->state.lastPrim = LaunchEffectPrim(61, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::Guard, [0.0, 0.0, 0.0]);

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.total_texture = 2;
    state.texture = Some(name);
    state.texture_secondary = Some("effect\\a01.bmp");
    state.size = 11.0;

    state.slots[3].life = 0;
    state.slots[2].life = 0;
    state.slots[1].life = 0;

    let slot = &mut state.slots[0];

    slot.life = 1;
    slot.process = 0;
    slot.max_height = 10.0;
    slot.alpha_b = 0;
    slot.distance = 1.0;
    slot.height[0] = 0.0;
    slot.flag1[0] = 1;
}

impl PortedEffect for Guard2 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        black_maximize(runtime, "effect\\a01.bmp");

        if (10..=30).contains(&runtime.frame) {
            if runtime.frame % 2 == 0 {
                if (runtime.parent_flags & 1) == 0 {
                    runtime.add_parent_flags(1);
                }

                runtime.set_parent_color(-1, 250, 250, 250);
            } else {
                runtime.set_parent_color(-1, 255, 255, 255);
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::step;

    #[test]
    fn guard_two_is_registered_with_lifetime_200() {
        assert_eq!(Guard2::new(200).lifetime(), 200);
    }

    #[test]
    fn frame_zero_spawns_the_cage_and_plays_the_sword_sound() {
        let mut effect = Guard2::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\black_maximize_power_sword_bic.wav".to_string()]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::Guard);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.texture, Some("effect\\a01.bmp"));
        assert_eq!(state.texture_secondary, Some("effect\\a01.bmp"));
        assert_eq!(state.total_texture, 2);
        assert_eq!(state.size, 11.0);

        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        assert_eq!(slot.max_height, 10.0);
        assert_eq!(slot.distance, 1.0);
        assert_eq!(slot.flag1[0], 1);
        assert_eq!(state.slots[1].life, 0);
        assert_eq!(state.slots[3].life, 0);
    }

    #[test]
    fn the_even_tint_frames_request_the_additive_body_flag() {
        let mut effect = Guard2::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        runtime.frame = 10;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.parent_color, Some([-1, 250, 250, 250]));
        assert_eq!(runtime.parent_flags & 1, 1);
    }

    #[test]
    fn the_odd_tint_frames_use_white_without_the_flag() {
        let mut effect = Guard2::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        runtime.frame = 11;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));
        assert_eq!(runtime.parent_flags & 1, 0);
        assert_eq!(runtime.primitives.len(), 0, "the cage only spawns on frame zero");
    }

    #[test]
    fn the_tint_window_is_frames_ten_to_thirty() {
        let mut effect = Guard2::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);
        runtime.frame = 9;

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_color, None);

        runtime.frame = 31;
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_color, None);
    }
}
