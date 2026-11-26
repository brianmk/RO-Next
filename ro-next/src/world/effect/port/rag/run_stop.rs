//! `CRagEffect::RunStop(short angle)` - `RagEffect.cpp:2593-2617`.
//!
//! Dispatch: `case EF_STOPEFFECT:` (444) - `RagEffect.cpp:25147-25154`:
//!
//! ```cpp
//! case EF_STOPEFFECT:
//!   if (this->state.stateCnt == 0) {
//!     this->render.position = this->state.parent->render.position;
//!     PlayEffectWaveInline(*this, "effect\\T_효과음1.wav");
//!   }
//!   this->RunStop(0);
//!   this->RunStop(45);
//!   break;
//! ```
//!
//! Lifetime 100 (`EffectTypes.cpp:453`). The dispatch is the body plus that one
//! inline wave, so `EF_STOPEFFECT` is registered with [`StopEffect`].
//!
//! `RunStop` gates itself on `stateCnt == 0`, so the two calls (angles 0 and
//! 45) launch their `PP_SLASH1` (prim 75, `primitive/slash1.rs`) on frame 0
//! only. Each is `effect\magic_blue.tga`, `size = 4`, four blades 90 degrees
//! apart with `distance = 2`, `height[0] = 1`, `maxHeight = 0` and `flag1[0] =
//! 1` (the fast growth variant with the 2-unit height cap). The renderer is the
//! shared [`render_slash_1`](super::super::render_slots::render_slash_1).
//!
//! The original copies `state.parent->render.position` into `render.position`
//! on every frame, before the gate; the port's [`EffectRuntime::position`] is
//! already that anchor. `PlayEffectWaveInline` is listener-relative at height
//! zero, so it goes through [`EffectRuntime::play_sound`].

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// The `EF_STOPEFFECT` body.
#[derive(Debug)]
pub struct StopEffect {
    lifetime: i32,
}

impl StopEffect {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }

    /// `CRagEffect::RunStop` - `RagEffect.cpp:2593-2617`.
    fn run_stop(runtime: &mut EffectRuntime, angle: i16) {
        // `CopyVector(this->render.position, this->state.parent->render.position);`
        // (represented by the runtime anchor, see the module docs)

        // `if (this->state.stateCnt != 0) return true;`
        if runtime.frame != 0 {
            return;
        }

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
            slot.distance = 2.0;
            slot.height[0] = 1.0;
            slot.max_height = 0.0;
            slot.rot_start = index * 90 + angle;
            slot.alpha_b = 0;
            slot.flag1[0] = 1;
        }
    }
}

impl PortedEffect for StopEffect {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    /// `case EF_STOPEFFECT:` - `RagEffect.cpp:25147-25154`.
    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            // `this->render.position = this->state.parent->render.position;`
            // (the runtime anchor already is the parent position)
            runtime.play_sound("effect\\T_효과음1.wav");
        }

        Self::run_stop(runtime, 0);
        Self::run_stop(runtime, 45);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn frame_zero_plays_the_wave_and_launches_both_rings() {
        let mut effect = StopEffect::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 13);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\T_효과음1.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 2);

        for (index, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            let angle = if index == 0 { 0 } else { 45 };

            assert_eq!(state.count, PrimitiveId::Slash1);
            assert_eq!(state.render_flag, 1 | 4);
            assert_eq!(state.duration, 100);
            assert_eq!(state.texture, Some("effect\\magic_blue.tga"));
            assert_eq!(state.size, 4.0);

            for (slot_index, slot) in state.slots.iter().enumerate() {
                // `flag1[0] = 1`: distance +1, maxHeight +0.2 (cap 2), alpha +5.
                assert_eq!(slot.life, 1);
                assert_eq!(slot.process, 1);
                assert_eq!(slot.distance, 3.0);
                assert_eq!(slot.height[0], 2.0);
                assert_eq!(slot.max_height, 0.2);
                assert_eq!(slot.rot_start, slot_index as i16 * 90 + angle);
                assert_eq!(slot.alpha_b, 5);
                assert_eq!(slot.flag1[0], 1);
            }
        }
    }

    #[test]
    fn later_frames_do_not_spawn_or_replay_the_wave() {
        let mut effect = StopEffect::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 13);

        for _ in 0..10 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2);
        assert_eq!(runtime.sounds.len(), 1, "PlayEffectWaveInline is frame-0 only");

        // Ten primitive passes: process 10, distance 2 + 10.
        assert_eq!(runtime.primitives[0].state.slots[0].process, 10);
        assert_eq!(runtime.primitives[0].state.slots[0].distance, 12.0);
        // `maxHeight += 0.2f` accumulates, so compare against the same sum.
        let mut expected_max_height = 0.0f32;
        for _ in 0..10 {
            expected_max_height += 0.2;
        }
        assert_eq!(runtime.primitives[0].state.slots[0].max_height, expected_max_height);
        // The alpha ramp stopped after process 7 (25); the 10th frame crosses
        // `height[0] > 10` (1 + 10) and starts the -2 fade.
        assert_eq!(runtime.primitives[0].state.slots[0].alpha_b, 23);
    }
}
