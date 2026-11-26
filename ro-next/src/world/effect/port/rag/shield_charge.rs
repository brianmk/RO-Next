//! `EF_SHIELDCHARGE` - `RagEffect.cpp:25344-25346`.
//!
//! The dispatch calls `ProcessEZ2STRC65300()`, **not**
//! `CRagEffect::ShieldCharge()`:
//!
//! ```cpp
//! case EF_SHIELDCHARGE:
//!   if (!this->ProcessEZ2STRC65300()) {
//!     return false;
//!   }
//!   break;
//! ```
//!
//! and the wrapper is `ProcessEZ2STR(0.0f, 0.0f, 0.0f)`
//! (`RagEffect.cpp:1884-1885`). The separately defined
//! `CRagEffect::ShieldCharge()` (`RagEffect.cpp:1562-1563`) passes
//! `(0.0f, -80.0f, 0.0f)`, but nothing in the dispatch calls it - keeping the
//! dispatch's `(0, 0, 0)` is the faithful port.
//!
//! The `.str` resource is `shield_charge.str`
//! (`RagEffectInitialization.cpp:365-366`); the lifetime is 499
//! (`EffectTypes.cpp:255`).

use super::super::runtime::{EffectRuntime, PortedEffect};
use super::super::str_player::Clip;

#[derive(Debug, Default)]
pub struct ShieldCharge {
    clips: Vec<Clip>,
}

impl ShieldCharge {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for ShieldCharge {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:255` - `result[EF_SHIELDCHARGE] = 499`.
        499
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 && runtime.str_player.is_none() && !self.clips.is_empty() {
            // The holder opens `shield_charge.str` before the first step; the
            // constructor clips are the test fallback.
            runtime.open_str(std::mem::take(&mut self.clips));
        }

        // `ProcessEZ2STRC65300()` = `ProcessEZ2STR(0.0f, 0.0f, 0.0f)`.
        // The runtime's default `(0, -80, 0)` is overwritten here with the
        // official `(0, 0, 0)` of this dispatch.
        runtime.process_str_with([0.0, 0.0, 0.0])
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
    fn shield_charge_uses_the_dispatch_offset_not_the_unused_method_offset() {
        let mut effect = ShieldCharge::new(clip());
        let mut runtime = EffectRuntime::new(effect.lifetime(), [0.0, 0.0], 1);

        step(&mut effect, &mut runtime);

        // `ProcessEZ2STRC65300` = `(0, 0, 0)`, not the `(0, -80, 0)` of the
        // uncalled `CRagEffect::ShieldCharge` at `RagEffect.cpp:1562`.
        assert_eq!(runtime.str_offset, [0.0, 0.0, 0.0]);
        assert_eq!(effect.lifetime(), 499);
    }

    #[test]
    fn shield_charge_retires_when_the_clip_ends() {
        // Two keys at frames 0 and 3: `ProcessEZ2STR` returns false on frame 4.
        let mut effect = ShieldCharge::new(vec![Clip {
            texture_count: 1,
            frames: vec![
                KeyFrame {
                    frame: 0,
                    transform: Transform::default(),
                },
                KeyFrame {
                    frame: 3,
                    transform: Transform {
                        x: 10.0,
                        ..Transform::default()
                    },
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
