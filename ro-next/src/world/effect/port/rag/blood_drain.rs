//! `CRagEffect::BloodDrain` - `RagEffect.cpp:284-321`, dispatch
//! `:24741-24743`, and `CRagEffect::RedParticleSpray` - `:12973-13008`,
//! dispatch `:24738-24740`.
//!
//! `EF_ENERGYDRAIN` (217) runs `BloodDrain`: three twelve-segment
//! `PP_3DPARTICLESPLINE` blue trails fanned by `roll = -90/0/90`.
//!
//! `EF_BLOODDRAIN` (216) runs `RedParticleSpray(3)`: the same three-trail fan
//! in red, seven segments each, rolled by `index * 90 - 90`.
//!
//! Both draw no random numbers, so the shared `EffectRng` sequence is
//! untouched.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct BloodDrain {
    lifetime: i32,
}

impl BloodDrain {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for BloodDrain {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();
        let target = EffectRuntime::official_position(runtime.target_offset);
        let position = EffectRuntime::official_position(runtime.position);
        let radius = runtime.calc_dist(target[0], target[2]);
        let face = if target[0] != position[0] || target[2] != position[2] {
            EffectRuntime::heading(position[0], position[2], target[0], target[2])
        } else {
            0.0
        };

        let mut angle = -90;

        while angle < 180 {
            let spline = runtime.launch(PrimitiveId::ParticleSpline, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spline);

            state.duration = 60;
            state.delta_pos2 = [0.0, -10.0, 0.0];
            state.radius = radius;
            state.render_angle = face;
            state.longitude = -state.render_angle;
            let speed = state.radius / state.duration as f32;
            state.speed = speed + speed;
            state.grav_speed = 2.0;
            let rise = -(state.grav_speed / state.duration as f32);
            state.grav_accel = rise + rise;
            state.lati_speed = -2.0;
            let spread = -(state.lati_speed / state.duration as f32);
            state.lati_accel = spread + spread;
            state.roll = angle as f32;
            state.transform.make_y_rotation(angles.radian(state.longitude as i32));
            state.delta_pos3 = matrix_mult([0.0, 0.0, 1.0], &state.transform.values);
            state.rot_axis = state.delta_pos3;
            state.red = 100;
            state.green = 100;
            state.blue = 250;
            state.size = 1.7;
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.num_segment = 7;
            state.init_segment();

            angle += 90;
        }

        true
    }
}

#[derive(Debug)]
pub struct RedParticleSpray {
    count: i32,
    lifetime: i32,
}

impl RedParticleSpray {
    pub fn new(count: i32, lifetime: i32) -> Self {
        Self { count, lifetime }
    }
}

impl PortedEffect for RedParticleSpray {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();
        let target = EffectRuntime::official_position(runtime.target_offset);
        let position = EffectRuntime::official_position(runtime.position);
        let radius = runtime.calc_dist(target[0], target[2]);
        let face = if target[0] != position[0] || target[2] != position[2] {
            EffectRuntime::heading(position[0], position[2], target[0], target[2])
        } else {
            0.0
        };

        for index in 0..self.count {
            let spline = runtime.launch(PrimitiveId::ParticleSpline, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spline);

            state.duration = 60;
            state.delta_pos2 = [0.0, -10.0, 0.0];
            state.radius = radius;
            state.render_angle = face;
            state.longitude = -state.render_angle;
            let speed = state.radius / state.duration as f32;
            state.speed = speed + speed;
            state.grav_speed = 2.0;
            let rise = -(state.grav_speed / state.duration as f32);
            state.grav_accel = rise + rise;
            state.lati_speed = -2.0;
            let spread = -(state.lati_speed / state.duration as f32);
            state.lati_accel = spread + spread;
            state.roll = (index * 90 - 90) as f32;
            state.transform.make_y_rotation(angles.radian(state.longitude as i32));
            state.delta_pos3 = matrix_mult([0.0, 0.0, 1.0], &state.transform.values);
            state.rot_axis = state.delta_pos3;
            state.red = 250;
            state.green = 100;
            state.blue = 100;
            state.size = 1.7;
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.num_segment = 7;
            state.init_segment();
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    fn runtime(lifetime: i32, seed: u64) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(lifetime, [0.0, 0.0], seed);
        runtime.position = [10.0, 0.0, 10.0];
        runtime.target_offset = [30.0, 0.0, 10.0];

        runtime
    }

    #[test]
    fn blood_drain_spawns_three_blue_trails_at_minus_ninety_zero_ninety() {
        let mut body = BloodDrain::new(200);
        let mut runtime = runtime(200, 7);

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);

        for (index, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::ParticleSpline);
            assert_eq!(state.duration, 60);
            assert_eq!(state.roll, (index as i32 * 90 - 90) as f32);
            assert_eq!((state.red, state.green, state.blue), (100, 100, 250));
            assert_eq!(state.size, 1.7);
            assert_eq!(state.num_segment, 7);
            assert_eq!(state.sprite, Some("이팩트\\particle1.spr"));
            assert!((state.radius - 20.0).abs() < 1e-5);
            // radius / duration, then doubled.
            assert!((state.speed - 20.0 / 60.0 * 2.0).abs() < 1e-5);
        }
    }

    #[test]
    fn red_particle_spray_spawns_three_red_trails_for_ef_blooddrain() {
        let mut body = RedParticleSpray::new(3, 200);
        let mut runtime = runtime(200, 11);

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);

        for (index, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::ParticleSpline);
            assert_eq!(state.roll, (index as i32 * 90 - 90) as f32);
            assert_eq!((state.red, state.green, state.blue), (250, 100, 100));
            assert_eq!(state.size, 1.7);
            assert_eq!(state.num_segment, 7);
        }
    }

    #[test]
    fn blood_drain_does_not_draw_random_numbers() {
        let mut body = BloodDrain::new(200);
        let mut runtime = runtime(200, 13);
        let mut expected = EffectRuntime::new(200, [0.0, 0.0], 13);
        // The body must leave the sequence exactly where it started.
        assert_eq!(runtime.rng.clamped_random(1000), expected.rng.clamped_random(1000));

        step(&mut body, &mut runtime);

        assert_eq!(runtime.rng.clamped_random(1000), expected.rng.clamped_random(1000));
    }
}
