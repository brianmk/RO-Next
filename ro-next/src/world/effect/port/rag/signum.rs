//! `CRagEffect::Signum` - `RagEffect.cpp:2107-2154`, effect id `EF_SIGNUM`.
//!
//! At frame 30 the sign of the cross lands: the same ring+rays burst as Bash
//! (with bigger sparks) and two sounds. The effect is otherwise carried by its
//! `.str` resource, and ends when that resource finishes.

use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};
use super::super::str_player::Clip;

#[derive(Debug, Default)]
pub struct Signum {
    clips: Vec<Clip>,
    opened: bool,
}

impl Signum {
    pub fn new(clips: Vec<Clip>) -> Self {
        Self { clips, opened: false }
    }
}

impl PortedEffect for Signum {
    fn lifetime(&self) -> i32 {
        999
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 && !self.opened {
            runtime.open_str(std::mem::take(&mut self.clips));
            self.opened = true;
        }

        if runtime.frame == 30 {
            runtime.play_sound("effect\\EF_Signum.wav");
            runtime.play_sound("effect\\EF_Bash.wav");

            let ring = runtime.launch(PrimitiveId::Circle2d, [0.0, 0.0, 0.0]);
            {
                let state = runtime.primitive_mut(ring);
                state.duration = 40;
                state.pattern |= 1;
                state.delta_pos2[1] = -40.0;
                state.radius = 100.0;
                state.alpha = 0.0;
                state.max_alpha = 170.0;
                state.alpha_speed = state.max_alpha / 6.0;
                // `duration - duration / 3` with integer division: 40 - 13.
                state.fade_out_cnt = state.duration - state.duration / 3;
                state.texture = Some("effect\\alpha_down.tga");
            }

            for _ in 0..20 {
                let longitude = runtime.rng.clamped_random(360) as f32;
                let long_speed = (runtime.rng.clamped_random(60) + 10) as f32 / 10.0;
                let arc_angle = (runtime.rng.clamped_random(25) + 15) as f32 / 10.0;
                let height_size = (runtime.rng.clamped_random(60) + 30) as f32;
                let height_speed = (runtime.rng.clamped_random(35) + 95) as f32 / 10.0;

                let ray = runtime.launch(PrimitiveId::Flash2d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(ray);
                state.duration = 35;
                state.delta_pos2[1] = -40.0;
                state.longitude = longitude;
                state.long_speed = long_speed;
                state.long_accel = -(state.long_speed / state.duration as f32) / 1.5;
                state.arc_angle = arc_angle;
                state.height_size = height_size;
                state.height_speed = height_speed;
                state.alpha = 0.0;
                state.max_alpha = 200.0;
                state.alpha_speed = state.max_alpha / 10.0;
                state.texture = Some("EFFECT\\alpha_center.tga");
            }
        }

        runtime.process_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{step, EffectRuntime};
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
        EffectRuntime::new(999, [0.0, 0.0], 11)
    }

    #[test]
    fn ring_and_rays_land_on_frame_thirty() {
        let mut effect = Signum::new(clips());
        let mut runtime = runtime();

        while runtime.frame < 30 {
            step(&mut effect, &mut runtime);
            assert!(runtime.primitives.is_empty(), "nothing before frame 30");
        }

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 21);
        assert_eq!(runtime.sounds, vec!["effect\\EF_Signum.wav".to_owned(), "effect\\EF_Bash.wav".to_owned()]);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.duration, 40);
        assert_eq!(ring.pattern & 1, 1);
        assert_eq!(ring.max_alpha, 170.0);
        assert_eq!(ring.fade_out_cnt, 27);

        let ray = &runtime.primitives[1].state;
        assert_eq!(ray.duration, 35);
        assert_eq!(ray.max_alpha, 200.0);
        assert_eq!(ray.fade_out_cnt, -1, "rays never fade out");
        assert!((0.5..4.0).contains(&ray.arc_angle));
        assert!((30.0..90.0).contains(&ray.height_size));
    }

    #[test]
    fn effect_lives_while_the_str_resource_lives() {
        let mut effect = Signum::new(clips());
        let mut runtime = runtime();

        for _ in 0..200 {
            assert!(step(&mut effect, &mut runtime));
        }
    }
}
