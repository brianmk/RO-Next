//! `EF_NAPALMVALCAN` (401) - `RagEffect.cpp:23293-23301`.
//!
//! The Volcano hit flash. On frames 20/30/40/50/60 the original re-anchors on
//! the caster, plays `EF_NapalmBeat.wav` and calls `Hit2(0)`.
//!
//! **The `Hit2(0)` call is dead.** `CRagEffect::Hit2` (`RagEffect.cpp:807-812`)
//! only spawns its shards when `state.stateCnt == 0`; every Volcano call site
//! is at frame 20 or later, so `Hit2` projects the anchor and returns without
//! touching `LaunchEffectPrim`. The port keeps the call - [`Hit2`] is the
//! already-ported body - so the deadness is visible and documented instead of
//! silently dropping the branch. The effect therefore has **no geometry**; only
//! the sound channel is observable.

use super::super::runtime::{EffectRuntime, PortedEffect};
use super::hit::Hit2;

#[derive(Debug)]
pub struct NapalmValcan {
    lifetime: i32,
    hit2: Hit2,
}

impl NapalmValcan {
    pub fn new(lifetime: i32) -> Self {
        Self {
            lifetime,
            hit2: Hit2::new(0),
        }
    }
}

impl PortedEffect for NapalmValcan {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        // `if (stateCnt == 20 || stateCnt == 30 || stateCnt == 40 ||
        //      stateCnt == 50 || stateCnt == 60) { ... }`
        if frame == 20 || frame == 30 || frame == 40 || frame == 50 || frame == 60 {
            // `this->render.position = this->state.parent->render.position;`
            runtime.copy_parent_position();
            // `PlayEffectWaveInline(*this, "effect\\EF_NapalmBeat.wav");`
            runtime.play_sound("effect\\EF_NapalmBeat.wav");
            // `this->Hit2(0);` - see the module docs: dead at frame != 0.
            self.hit2.on_process(runtime);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn plays_the_wave_on_the_five_hit_frames_only() {
        let mut effect = NapalmValcan::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        for _ in 0..61 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.sounds, vec![
            "effect\\EF_NapalmBeat.wav".to_owned(),
            "effect\\EF_NapalmBeat.wav".to_owned(),
            "effect\\EF_NapalmBeat.wav".to_owned(),
            "effect\\EF_NapalmBeat.wav".to_owned(),
            "effect\\EF_NapalmBeat.wav".to_owned(),
        ]);
    }

    #[test]
    fn the_hit2_call_spawns_no_geometry() {
        let mut effect = NapalmValcan::new(200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 1);

        for _ in 0..61 {
            step(&mut effect, &mut runtime);
        }

        assert!(runtime.primitives.is_empty(), "Hit2 is dead after frame 0");
    }
}
