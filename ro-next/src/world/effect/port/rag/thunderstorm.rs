//! `EF_THUNDERSTORM` - `RagEffect.cpp:25165-25172`.
//!
//! The body is inline in the `CRagEffect::OnProcess` dispatch (there is no
//! separately named `CRagEffect::Thunderstorm` in the decompilation):
//!
//! ```cpp
//! case EF_THUNDERSTORM:
//!   if (this->state.stateCnt == 0) {
//!     PlayEffectWaveInline(*this, "effect\\magician_thunderstorm.wav");
//!   }
//!   if (!this->ProcessEZ2STR(0.0f, -80.0f, 0.0f)) {
//!     return false;
//!   }
//!   break;
//! ```
//!
//! Frame 0 plays the wave; every frame advances the checked `.str` clip, and
//! the effect retires when the clip reports finished. The resource is
//! `ThunderStorm.str` (`RagEffectInitialization.cpp:39-40`); the lifetime is
//! 104 (`EffectTypes.cpp:39`).

use super::super::runtime::{DEFAULT_STR_OFFSET, EffectRuntime, PortedEffect};
use super::super::str_player::Clip;

#[derive(Debug, Default)]
pub struct Thunderstorm {
    clips: Vec<Clip>,
}

impl Thunderstorm {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Thunderstorm {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:39` - `result[EF_THUNDERSTORM] = 104`.
        104
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 && runtime.str_player.is_none() && !self.clips.is_empty() {
            // The holder opens `ThunderStorm.str` before the first step; the
            // constructor clips are the test fallback.
            runtime.open_str(std::mem::take(&mut self.clips));
        }

        if runtime.frame == 0 {
            runtime.play_sound("effect\\magician_thunderstorm.wav");
        }

        // `if (!this->ProcessEZ2STR(0.0f, -80.0f, 0.0f)) return false;`
        runtime.process_str_with(DEFAULT_STR_OFFSET)
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
    fn frame_zero_plays_the_wave_and_uses_the_default_offset() {
        let mut effect = Thunderstorm::new(clip());
        let mut runtime = EffectRuntime::new(effect.lifetime(), [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\magician_thunderstorm.wav".to_owned()]);
        assert_eq!(runtime.str_offset, [0.0, -80.0, 0.0]);
        assert_eq!(effect.lifetime(), 104);
    }

    #[test]
    fn the_checked_clip_retires_the_effect() {
        // Two keys at frames 0 and 3: `ProcessEZ2STR` reports finished on
        // frame 4 and the body forwards the `false` (unlike RejectSword, which
        // discards it).
        let mut effect = Thunderstorm::new(vec![Clip {
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

        for _ in 0..4 {
            assert!(step(&mut effect, &mut runtime));
        }

        assert!(!step(&mut effect, &mut runtime));
    }
}
