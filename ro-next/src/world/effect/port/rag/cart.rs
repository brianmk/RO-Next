//! `CRagEffect::CartRevolution` - `RagEffect.cpp:2234-2273`, effect id
//! `EF_CARTREVOLUTION`.
//!
//! Two ring/blast pairs at frames 7 and 20, plus the effect's `.str` resource
//! (`ProcessEZ2STR(0, -80, 0)`), which also owns the ending: the body retires
//! at frame 300.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};
use super::super::str_player::Clip;

#[derive(Debug, Default)]
pub struct CartRevolution {
    clips: Vec<Clip>,
    opened: bool,
}

impl CartRevolution {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips, opened: false }
    }
}

impl PortedEffect for CartRevolution {
    fn lifetime(&self) -> i32 {
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 && !self.opened {
            // The holder opens `CartRevolution.str` before the first step; only
            // fall back to constructor clips when no asset layer supplied them.
            if runtime.str_player.is_none() && !self.clips.is_empty() {
                runtime.open_str(std::mem::take(&mut self.clips));
            }

            self.opened = true;
        }

        if runtime.frame == 7 || runtime.frame == 20 {
            runtime.play_sound("effect\\EF_MagnumBreak.wav");

            let ring = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
            {
                let state = runtime.primitive_mut(ring);
                state.duration = 20;
                state.render_flag |= 0x200;
                state.max_alpha = 180.0;
                state.latitude = 90.0;
                state.radius_speed = 1.75;
                state.radius_accel = -(state.radius_speed / state.duration as f32) * 0.5;
                state.inner_size = 5.0;
                state.alpha = 0.0;
                state.alpha_speed = state.max_alpha / 15.0;
                state.fade_out_cnt = state.duration - 15;
                state.init_segment();
                state.texture = Some("effect\\ring_yellow.tga");
            }

            let blast = runtime.launch(PrimitiveId::Sphere3d, [0.0, 0.0, 0.0]);
            {
                let state = runtime.primitive_mut(blast);
                state.duration = 20;
                state.radius_speed = 1.35;
                state.radius_accel = -(state.radius_speed / state.duration as f32) * 0.5;
                state.long_speed = 3.0;
                state.alpha = 0.0;
                state.max_alpha = 240.0;
                state.alpha_speed = state.max_alpha / 7.0;
                state.fade_out_cnt = state.duration - 15;
                state.texture = Some("effect\\대폭발.tga");
            }
        }

        if runtime.frame >= 300 {
            return false;
        }

        runtime.process_str();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};
    use crate::world::effect::port::str_player::{KeyFrame, Transform};

    fn clips() -> Vec<Clip> {
        vec![Clip {
            texture_count: 1,
            frames: vec![KeyFrame {
                frame: 0,
                transform: Transform::default(),
            }],
        }]
    }

    fn runtime() -> EffectRuntime {
        EffectRuntime::new(999, [0.0, 0.0], 7)
    }

    #[test]
    fn spawns_at_frames_seven_and_twenty() {
        let mut effect = CartRevolution::new(clips());
        let mut runtime = runtime();

        while runtime.frame < 8 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 2, "ring + blast at frame 7");

        while runtime.frame < 21 {
            step(&mut effect, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 4, "second pair at frame 20");
        assert_eq!(runtime.sounds.len(), 2, "one sound per pair");
    }

    #[test]
    fn ring_matches_the_original_constants() {
        let mut effect = CartRevolution::new(clips());
        let mut runtime = runtime();

        while runtime.frame < 8 {
            step(&mut effect, &mut runtime);
        }

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.duration, 20);
        assert_eq!(ring.max_alpha, 180.0);
        assert_eq!(ring.latitude, 90.0);
        assert_eq!(ring.inner_size, 5.0);
        assert_eq!(ring.fade_out_cnt, 5);
        assert!((ring.alpha_speed - 12.0).abs() < 1e-4);

        let blast = &runtime.primitives[1].state;
        assert_eq!(blast.max_alpha, 240.0);
        assert!((blast.alpha_speed - 240.0 / 7.0).abs() < 1e-4);
        assert_eq!(blast.long_speed, 3.0);
    }

    #[test]
    fn retires_at_frame_three_hundred() {
        let mut effect = CartRevolution::new(clips());
        let mut runtime = runtime();

        let mut frames = 0;
        while step(&mut effect, &mut runtime) {
            frames += 1;
            if frames > 400 {
                panic!("effect did not retire");
            }
        }

        assert_eq!(runtime.frame, 300);
    }

    #[test]
    fn opens_the_str_resource() {
        let mut effect = CartRevolution::new(clips());
        let mut runtime = runtime();

        step(&mut effect, &mut runtime);

        assert!(runtime.str_player.is_some());
        assert_eq!(runtime.str_clips.len(), 1);
    }
}
