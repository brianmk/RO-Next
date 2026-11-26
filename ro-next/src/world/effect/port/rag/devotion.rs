//! `EF_DEVOTION` - `RagEffect.cpp:24604-24607`.
//!
//! ```cpp
//! case EF_DEVOTION:
//!   if (!this->ProcessEZ2STR(0.0f, -20.0f, 0.0f)) {
//!     return false;
//!   }
//!   break;
//! ```
//!
//! The `y` offset is **-20**, not the default -80. The `.str` resource is
//! `devotion.str` (`RagEffectInitialization.cpp:368-369`); the lifetime is 150
//! (`EffectTypes.cpp:260`).

use super::super::runtime::{EffectRuntime, PortedEffect};
use super::super::str_player::Clip;

#[derive(Debug, Default)]
pub struct Devotion {
    clips: Vec<Clip>,
}

impl Devotion {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for Devotion {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:260` - `result[EF_DEVOTION] = 150`.
        150
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 && runtime.str_player.is_none() && !self.clips.is_empty() {
            runtime.open_str(std::mem::take(&mut self.clips));
        }

        runtime.process_str_with([0.0, -20.0, 0.0])
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
    fn devotion_uses_the_official_minus_twenty_offset() {
        let mut effect = Devotion::new(clip());
        let mut runtime = EffectRuntime::new(effect.lifetime(), [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.str_offset, [0.0, -20.0, 0.0]);
        assert_eq!(effect.lifetime(), 150);
    }

    #[test]
    fn devotion_retires_when_the_clip_ends() {
        let mut effect = Devotion::new(vec![Clip {
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
