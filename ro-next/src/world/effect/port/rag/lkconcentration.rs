//! `EF_LKCONCENTRATION` - `RagEffect.cpp:10377-10382`.
//!
//! ```cpp
//! case EF_LKCONCENTRATION:
//!   if (this->state.stateCnt == 0) {
//!     PlayEffectWaveInline(*this, "effect\\knight_twohandquicken.wav");
//!   }
//!   this->state.parent->SetColor(-1, 255, 255, 160);
//!   this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);
//!   break;
//! ```
//!
//! Both the sound and the colour are re-applied frame by frame; the
//! `ProcessEZ2STR` result is discarded, so the body always returns `true`
//! until the 99999-frame lifetime expires. The `.str` resource is
//! `TwoHand.str` (`RagEffectInitialization.cpp:439-442`); the lifetime is
//! 99999 (`EffectTypes.cpp:378`).

use super::super::runtime::{DEFAULT_STR_OFFSET, EffectRuntime, PortedEffect};
use super::super::str_player::Clip;

#[derive(Debug, Default)]
pub struct LkConcentration {
    clips: Vec<Clip>,
}

impl LkConcentration {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for LkConcentration {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:378` - `result[EF_LKCONCENTRATION] = 99999`.
        99999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 && runtime.str_player.is_none() && !self.clips.is_empty() {
            runtime.open_str(std::mem::take(&mut self.clips));
        }

        if runtime.frame == 0 {
            runtime.play_sound("effect\\knight_twohandquicken.wav");
        }

        // `this->state.parent->SetColor(-1, 255, 255, 160);`
        runtime.set_parent_color(-1, 255, 255, 160);

        // `this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);` - the return value is
        // discarded by the official case, so the effect stays alive.
        let _ = runtime.process_str_with(DEFAULT_STR_OFFSET);

        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::super::runtime::{EffectRuntime, step};
    use super::super::super::str_player::{KeyFrame, Transform};
    use super::*;

    fn clip() -> Vec<Clip> {
        vec![Clip {
            texture_count: 1,
            frames: vec![KeyFrame {
                frame: 0,
                transform: Transform::default(),
            }],
        }]
    }

    #[test]
    fn frame_zero_plays_the_wave_and_every_frame_sets_the_tint() {
        let mut effect = LkConcentration::new(clip());
        let mut runtime = EffectRuntime::new(effect.lifetime(), [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\knight_twohandquicken.wav".to_owned()]);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 160]));
        assert_eq!(runtime.str_offset, [0.0, -80.0, 0.0]);

        // The `SetColor` call is per frame, not spawn-only.
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 160]));
        assert_eq!(runtime.sounds.len(), 1, "the wave is frame-0 only");
    }

    #[test]
    fn the_discarded_process_ez2str_result_does_not_retire_the_effect() {
        // Two keys at frames 0 and 3: the player reports "finished" on frame
        // 4, but the official `case` ignores it.
        let mut effect = LkConcentration::new(vec![Clip {
            texture_count: 1,
            frames: vec![
                KeyFrame {
                    frame: 0,
                    transform: Transform::default(),
                },
                KeyFrame {
                    frame: 3,
                    transform: Transform::default(),
                },
            ],
        }]);
        let mut runtime = EffectRuntime::new(effect.lifetime(), [0.0, 0.0], 1);

        for _ in 0..6 {
            assert!(step(&mut effect, &mut runtime), "frame {} retired", runtime.frame);
        }

        assert_eq!(runtime.frame, 6);
    }
}
