//! `CRagEffect::SoulStrike` - `RagEffect.cpp:1002-1087`, dispatch
//! `:23168-23170` (`EF_SOULSTRIKE`, 15) and `:23178-23180`
//! (`EF_SOULSTRIKE2`, 451).
//!
//! The bolt count comes from the effect's `state.count` (`AM_SETCOUNT`): the
//! switch arms select a fan `(total, base, step)` of 1..7 twelve-segment
//! `PP_3DPARTICLESPLINE` trails, one every eleven frames starting at frame 6
//! (`(stateCnt + 5) / 11`). The variant only selects `particle1` (the purple
//! Soul Strike) or `particle5` (the Soul Strike 2 sprite).
//!
//! Quirk kept: `count` 0 or above 7 leaves `total = 0`, so the gate
//! `(stateCnt + delay) / dist > total` rejects every frame and the effect emits
//! nothing.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct SoulStrike {
    second: bool,
    lifetime: i32,
}

impl SoulStrike {
    pub fn new(second: bool, lifetime: i32) -> Self {
        Self { second, lifetime }
    }
}

impl PortedEffect for SoulStrike {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let (mut base, mut step, mut total) = (0, 0, 0);

        match runtime.count {
            1 => {
                total = 1;
                base = 0;
                step = 1;
            }
            2 => {
                total = 2;
                step = 180;
                base = -90;
            }
            3 => {
                total = 3;
                step = 90;
                base = -90;
            }
            4 => {
                total = 4;
                step = 60;
                base = -90;
            }
            5 => {
                total = 5;
                step = 45;
                base = -90;
            }
            6 => {
                total = 6;
                step = 30;
                base = -90;
            }
            7 => {
                total = 7;
                step = 25;
                base = -90;
            }
            _ => {}
        }

        let dist = 11;
        let delay = 5;

        if (runtime.frame + delay) % dist != 0 || (runtime.frame + delay) / dist > total {
            return true;
        }

        let angle = base + (runtime.frame + delay) / dist * step;

        runtime.play_sound("effect\\EF_SoulStrike.wav");

        let angles = Angles::shared();
        let target = EffectRuntime::official_position(runtime.target_offset);
        let position = EffectRuntime::official_position(runtime.position);
        let radius = runtime.calc_dist(target[0], target[2]);
        let face = if target[0] != position[0] || target[2] != position[2] {
            EffectRuntime::heading(position[0], position[2], target[0], target[2])
        } else {
            0.0
        };

        let spline = runtime.launch(PrimitiveId::ParticleSpline, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spline);

        state.duration = 40;
        state.delta_pos2 = [0.0, -10.0, 0.0];
        state.radius = radius;
        state.render_angle = face;
        state.longitude = -state.render_angle;
        state.speed = state.radius / state.duration as f32;
        state.grav_speed = 3.5;
        let rise = -(state.grav_speed / state.duration as f32);
        state.grav_accel = rise + rise;
        state.lati_speed = -2.0;
        let spread = -(state.lati_speed / state.duration as f32);
        state.lati_accel = spread + spread;
        state.roll = angle as f32;
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));
        state.delta_pos3 = matrix_mult([0.0, 0.0, 1.0], &state.transform.values);
        state.rot_axis = state.delta_pos3;
        state.size = 2.75;

        if self.second {
            state.sprite = Some("이팩트\\particle5.spr");
            state.action = Some("이팩트\\particle5.act");
        } else {
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
        }

        state.num_segment = 12;
        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    fn runtime(count: i32) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 15);
        runtime.position = [10.0, 0.0, 10.0];
        runtime.target_offset = [30.0, 0.0, 10.0];
        runtime.count = count;

        runtime
    }

    #[test]
    fn soul_strike_fires_one_bolt_every_eleven_frames_from_frame_six() {
        let mut body = SoulStrike::new(false, 100);
        let mut runtime = runtime(1);

        // Frames 0..=5 draw nothing.
        for _ in 0..6 {
            step(&mut body, &mut runtime);
        }

        assert!(runtime.primitives.is_empty());

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        assert_eq!(runtime.sounds, vec!["effect\\EF_SoulStrike.wav".to_owned()]);

        let state = &runtime.primitives[0].state;
        assert_eq!(state.count, PrimitiveId::ParticleSpline);
        assert_eq!(state.duration, 40);
        assert_eq!(state.roll, 1.0);
        assert_eq!(state.size, 2.75);
        assert_eq!(state.num_segment, 12);
        assert_eq!(state.sprite, Some("이팩트\\particle1.spr"));

        // Count 1 stops after the first bolt.
        for _ in 0..11 {
            step(&mut body, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 1);
    }

    #[test]
    fn soul_strike_count_three_fans_minus_ninety_zero_ninety() {
        let mut body = SoulStrike::new(false, 100);
        let mut runtime = runtime(3);
        let mut rolls = Vec::new();

        for _ in 0..40 {
            let before = runtime.primitives.len();
            step(&mut body, &mut runtime);

            if runtime.primitives.len() > before {
                rolls.push(runtime.primitives.last().unwrap().state.roll);
            }
        }

        assert_eq!(rolls, vec![0.0, 90.0, 180.0]);
    }

    #[test]
    fn soul_strike_two_uses_particle5() {
        let mut body = SoulStrike::new(true, 100);
        let mut runtime = runtime(1);

        for _ in 0..7 {
            step(&mut body, &mut runtime);
        }

        let state = &runtime.primitives[0].state;
        assert_eq!(state.sprite, Some("이팩트\\particle5.spr"));
        assert_eq!(state.action, Some("이팩트\\particle5.act"));
    }

    #[test]
    fn soul_strike_count_zero_and_eight_emit_nothing() {
        for count in [0, 8] {
            let mut body = SoulStrike::new(false, 100);
            let mut runtime = runtime(count);

            for _ in 0..40 {
                step(&mut body, &mut runtime);
            }

            assert!(runtime.primitives.is_empty(), "count {count}");
        }
    }
}
