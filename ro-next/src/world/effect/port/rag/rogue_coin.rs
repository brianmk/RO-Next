//! `EF_RG_COIN2` dispatch block - `RagEffect.cpp:20997-21025`.
//!
//! Plays `복호격.wav` on frames 1, 11, 21 and 31 and calls `TexturedSprayKind`
//! on every frame. Because the spray helper bails unless the frame counter is
//! zero, **no spray is ever drawn**: the official effect is sound-only. The
//! port keeps the calls so the quirk is explicit.

use super::super::runtime::{EffectRuntime, PortedEffect};
use super::spray;

#[derive(Debug)]
pub struct RogueCoin {
    lifetime: i32,
}

impl RogueCoin {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for RogueCoin {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 1 || frame == 11 || frame == 21 || frame == 31 {
            runtime.play_sound("effect\\복호격.wav");
        }

        // Called every frame by the original; the helper is frame-gated.
        for _ in 0..4 {
            spray::spray_kind(runtime, "effect\\shield.bmp", 5.0, 0);
            spray::spray_kind(runtime, "effect\\sword.bmp", 9.0, 0);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};

    #[test]
    fn wave_plays_on_the_four_stages_and_no_spray_is_drawn() {
        let mut effect = RogueCoin::new(300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 341);

        for _ in 0..32 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(
            runtime.sounds,
            vec![
                "effect\\복호격.wav".to_owned(),
                "effect\\복호격.wav".to_owned(),
                "effect\\복호격.wav".to_owned(),
                "effect\\복호격.wav".to_owned(),
            ]
        );
        // Only frame 0 passes the spray gate (4 calls x 2 textures); every
        // later frame draws nothing because the helper is frame-gated.
        assert_eq!(runtime.primitives.len(), 8);
        assert!(runtime.primitives.iter().all(|p| p.state.texture.is_some()));
    }
}
