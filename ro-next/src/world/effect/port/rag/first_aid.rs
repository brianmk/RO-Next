//! `CRagEffect::FirstAid()` - `RagEffect.cpp:5123-5161`; dispatch
//! `case EF_FIRSTAID:` `RagEffect.cpp:10153-10154`.
//!
//! The `_heal_effect.wav` spark of `EF_FIRSTAID` (309, lifetime 299 at
//! `EffectTypes.cpp:318`). The body copies the parent anchor, returns on every
//! frame but frame 0, plays the sound at height 150 and arms one **prim 58**
//! (`PP_FIRSTAID`) primitive:
//!
//! * `renderFlag = RSF_ALPHA | RSF_CUSTOM_BLEND`, texture
//!   `effect\\pikapika2.bmp`, `size = 0`,
//! * `slot0.life = 1`, `process = 0`, `distance = 10`, `alphaB = 0`,
//! * `vecBNow = (anchor.x, anchor.y - 12, anchor.z)`,
//! * `height[6] = 0.02`, `flag1[0] = 0`, `flag1[2] = 1`,
//! * `fullDisplayAngle = ClampedRandom(360)` then `height[8] =
//!   ClampedRandom(360)` (that is the RNG order),
//! * slots 1..3 are killed and their `alphaB` cleared.
//!
//! The renderer is the shared
//! [`render_field_panel`](super::super::render_slots::render_field_panel)
//! (`RenderFieldPanel`, `EffectPrimitiveRendering.cpp:3312-3505`, dispatch
//! `:578`); the motion [`first_aid`](super::super::primitive::first_aid) spins
//! `height[8]` by 3 degrees per frame.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// The `EF_FIRSTAID` dispatch.
#[derive(Debug)]
pub struct FirstAid {
    lifetime: i32,
}

impl FirstAid {
    pub const fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

/// `CRagEffect::FirstAid()` - `RagEffect.cpp:5123-5161`.
fn first_aid(runtime: &mut EffectRuntime) {
    runtime.copy_parent_position();

    if runtime.frame != 0 {
        return;
    }

    runtime.play_sound_at_height("_heal_effect.wav", 150.0);

    let lifetime = runtime.lifetime;
    let position = EffectRuntime::official_position(runtime.position);

    // `this->state.lastPrim = LaunchEffectPrim(58, {0, 0, 0});`
    let index = runtime.launch(PrimitiveId::FirstAid, [0.0, 0.0, 0.0]);

    // RNG order: fullDisplayAngle (360), height[8] (360).
    let full_display_angle = runtime.rng.clamped_random(360) as i16;
    let height8 = runtime.rng.clamped_random(360) as f32;

    let state = runtime.primitive_mut(index);

    state.render_flag = 1 | 4;
    state.duration = lifetime;
    state.texture = Some("effect\\pikapika2.bmp");
    state.size = 0.0;

    let slot = &mut state.slots[0];

    slot.life = 1;
    slot.process = 0;
    slot.distance = 10.0;
    slot.height[8] = 0.0;
    slot.alpha_b = 0;
    slot.vec_b_now[0] = position[0];
    slot.vec_b_now[2] = position[2];
    slot.vec_b_now[1] = position[1] - 12.0;
    slot.height[6] = 0.02;
    slot.full_display_angle = full_display_angle;
    slot.flag1[0] = 0;
    slot.flag1[2] = 1;
    slot.height[8] = height8;

    state.slots[1].life = 0;
    state.slots[1].alpha_b = 0;
    state.slots[2].life = 0;
    state.slots[2].alpha_b = 0;
    state.slots[3].life = 0;
    state.slots[3].alpha_b = 0;
}

impl PortedEffect for FirstAid {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        first_aid(runtime);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::rng::EffectRng;
    use crate::world::effect::port::runtime::step;

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(299, [0.0, 0.0], 11);
        runtime.parent_position = Some([10.0, 20.0, 30.0]);
        runtime
    }

    #[test]
    fn spawns_the_spark_panel_and_plays_the_wave() {
        let mut effect = FirstAid::new(299);
        let mut runtime = runtime();

        let mut expected = EffectRng::new(11);
        let full_display_angle = expected.clamped_random(360) as i16;
        let height8 = expected.clamped_random(360) as f32;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.positional_sounds, vec![("_heal_effect.wav".to_owned(), [
            10.0, 150.0, 30.0
        ])]);
        assert_eq!(runtime.primitives.len(), 1);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::FirstAid);
        assert_eq!(state.render_flag, 5);
        assert_eq!(state.duration, 299);
        assert_eq!(state.texture, Some("effect\\pikapika2.bmp"));
        assert_eq!(state.size, 0.0);

        let slot = &state.slots[0];
        assert_eq!(slot.life, 1);
        // PrimFirstAid runs in the same step: process 1, alpha +4.
        assert_eq!(slot.process, 1);
        assert_eq!(slot.alpha_b, 4);
        assert_eq!(slot.distance, 10.0);
        // Official Y-down: the port anchor's y 20 is -20, then the -12 lift.
        assert_eq!(slot.vec_b_now, [10.0, -32.0, 30.0]);
        assert_eq!(slot.height[6], 0.02);
        assert_eq!(slot.full_display_angle, full_display_angle);
        assert_eq!(slot.flag1[0], 0);
        assert_eq!(slot.flag1[2], 1);
        // height[8] starts at the second draw and then spins by 3.
        assert_eq!(slot.height[8], (height8 + 3.0) % 360.0);

        for slot in &state.slots[1..] {
            assert_eq!(slot.life, 0);
            assert_eq!(slot.alpha_b, 0);
        }
    }

    #[test]
    fn later_frames_spawn_nothing() {
        let mut effect = FirstAid::new(299);
        let mut runtime = runtime();

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.positional_sounds.len(), 1);
    }

    #[test]
    fn the_anchor_follows_the_parent() {
        let mut effect = FirstAid::new(299);
        let mut runtime = runtime();

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.position, [10.0, 20.0, 30.0]);
    }
}
