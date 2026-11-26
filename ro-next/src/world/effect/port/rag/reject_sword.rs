//! `EF_REJECTSWORD` - `RagEffect.cpp:23337-23352`.
//!
//! ```cpp
//! case EF_REJECTSWORD:
//!   if (this->state.stateCnt == 0) {
//!     this->render.position = this->state.parent->render.position;
//!     PlayEffectWaveInline(*this, "effect\\kyrie_guard.wav");
//!   }
//!   if (this->state.stateCnt >= 0 && this->state.stateCnt < 30) {
//!     if ((this->state.parent->render.flags & 8) == 0) {
//!       this->state.parent->render.flags += 8;
//!     }
//!     if (this->state.stateCnt % 6 < 3) {
//!       this->state.parent->SetColor(-1, 100, 100, 100);
//!     } else {
//!       this->state.parent->SetColor(-1, 250, 250, 250);
//!     }
//!   }
//!   this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);
//!   break;
//! ```
//!
//! The blink cadence is the integer `stateCnt % 6 < 3` gate: dark on frames
//! 0-2, bright on 3-5, repeating through frame 29. Bit `8` is requested from
//! frame 0 through frame 29; from frame 30 on the body no longer writes either
//! the flag or the colour, and the actor's per-frame `render.flags` clear
//! handles the wave. The colour itself is left at its last value by the
//! official body (the port records the last `SetColor` request too); see
//! [`EffectRuntime::set_parent_color`] for the client-application status.
//!
//! The `.str` resource is `sword.str`
//! (`RagEffectInitialization.cpp:430-431`); the lifetime is 200
//! (`EffectTypes.cpp:401`).

use super::super::runtime::{DEFAULT_STR_OFFSET, EffectRuntime, PortedEffect};
use super::super::str_player::Clip;

#[derive(Debug, Default)]
pub struct RejectSword {
    clips: Vec<Clip>,
}

impl RejectSword {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips }
    }
}

impl PortedEffect for RejectSword {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:401` - `result[EF_REJECTSWORD] = 200`.
        200
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 && runtime.str_player.is_none() && !self.clips.is_empty() {
            runtime.open_str(std::mem::take(&mut self.clips));
        }

        if runtime.frame == 0 {
            // `this->render.position = this->state.parent->render.position;`
            // is the spawn anchor already; only the wave is observable here.
            runtime.play_sound("effect\\kyrie_guard.wav");
        }

        // The official condition is `stateCnt >= 0 && stateCnt < 30`; the
        // lower bound is redundant but kept verbatim.
        if runtime.frame >= 0 && runtime.frame < 30 {
            if runtime.parent_flags & 8 == 0 {
                runtime.add_parent_flags(8);
            }

            if runtime.frame % 6 < 3 {
                runtime.set_parent_color(-1, 100, 100, 100);
            } else {
                runtime.set_parent_color(-1, 250, 250, 250);
            }
        }

        // `this->ProcessEZ2STR(0.0f, -80.0f, 0.0f);` - return value discarded.
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
    fn the_six_frame_blink_cadence_matches_state_cnt_mod_six() {
        let mut effect = RejectSword::new(clip());
        let mut runtime = EffectRuntime::new(effect.lifetime(), [0.0, 0.0], 1);

        let mut seen = Vec::new();
        for frame in 0..30 {
            assert!(step(&mut effect, &mut runtime), "frame {frame} retired");
            assert_eq!(runtime.frame, frame + 1);
            assert_eq!(runtime.parent_flags & 8, 8, "flags bit 8 while {}", frame);
            seen.push(runtime.parent_color.expect("SetColor ran in the window"));
        }

        let expected: Vec<[i16; 4]> = (0..30)
            .map(|frame| if frame % 6 < 3 { [-1, 100, 100, 100] } else { [-1, 250, 250, 250] })
            .collect();

        assert_eq!(seen, expected);
    }

    #[test]
    fn the_wave_and_flag_stop_after_frame_twenty_nine() {
        let mut effect = RejectSword::new(clip());
        let mut runtime = EffectRuntime::new(effect.lifetime(), [0.0, 0.0], 1);

        for _ in 0..30 {
            step(&mut effect, &mut runtime);
        }

        // Frame 29 (the last window frame) selected the bright value.
        assert_eq!(runtime.parent_color, Some([-1, 250, 250, 250]));

        // Frame 30: no new `SetColor` and no flag request; the colour keeps
        // the official body's last value because the body never resets it.
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.parent_flags, 0, "begin_step cleared the per-frame flags");
        assert_eq!(runtime.parent_color, Some([-1, 250, 250, 250]));
        assert_eq!(runtime.sounds, vec!["effect\\kyrie_guard.wav".to_owned()], "frame-0 wave only");
    }
}
