//! `CRagEffect::EnergyDrain2` - `RagEffect.cpp:1336-1382`, dispatch
//! `:24744-24752`.
//!
//! `EF_ENERGYDRAIN2` (378): three twelve-segment `PP_3DPARTICLESPLINE`
//! trails aimed *away* from the target (`Face(2 * position - deltaPos)`) and
//! spawned with the half-target offset in `deltaPos2`. Each trail draws its
//! red/green from `ClampedRandom(81) + 160`, its size from
//! `ClampedRandom(6) * 0.1 + 1.5`, and glows blue.
//!
//! The dispatch wraps the body with the caster tint: frames 55..=65 request
//! `render.flags |= 1` and `SetColor(-1, 100, 100, 255)`
//! (`RagEffect.cpp:24745-24751`). The port records both on the runtime for
//! the client bridge.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct EnergyDrain2 {
    lifetime: i32,
}

impl EnergyDrain2 {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for EnergyDrain2 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame == 0 {
            let angles = Angles::shared();
            let target = runtime.target_offset;
            let position = runtime.position;
            let delta_x = target[0] - position[0];
            let delta_z = target[2] - position[2];

            let mut angle = -90;

            while angle < 180 {
                // Halfway between the anchor and the target, measured from the
                // (same) anchor the primitive was launched at.
                let mid_x = target[0] - (target[0] - position[0]) * 0.5;
                let mid_z = target[2] - (target[2] - position[2]) * 0.5;
                let radius = runtime.calc_dist(mid_x, mid_z);
                // `Face((position + position) - deltaPos)`: away from the
                // target.
                let face_x = (position[0] + position[0]) - target[0];
                let face_z = (position[2] + position[2]) - target[2];
                let face = if face_x != position[0] || face_z != position[2] {
                    EffectRuntime::heading(position[0], position[2], face_x, face_z)
                } else {
                    0.0
                };

                // `ClampedRandom` order per trail: red, green, size.
                let red = runtime.rng.clamped_random(81) + 160;
                let green = runtime.rng.clamped_random(81) + 160;
                let size = runtime.rng.clamped_random(6) as f32 * 0.1 + 1.5;

                let spline = runtime.launch(PrimitiveId::ParticleSpline, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(spline);

                state.duration = 60;
                state.delta_pos2 = [delta_x, -10.0, delta_z];
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
                state.red = red;
                state.green = green;
                state.blue = 255;
                state.size = size;
                state.sprite = Some("이팩트\\particle1.spr");
                state.action = Some("이팩트\\particle1.act");
                state.num_segment = 7;
                state.init_segment();

                angle += 90;
            }
        }

        if (55..=65).contains(&runtime.frame) {
            runtime.add_parent_flags(1);
            runtime.set_parent_color(-1, 100, 100, 255);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    fn runtime() -> EffectRuntime {
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 378);
        runtime.position = [10.0, 0.0, 10.0];
        runtime.target_offset = [30.0, 0.0, 10.0];

        runtime
    }

    #[test]
    fn energy_drain_two_spawns_three_trails_with_the_half_target_offset() {
        let mut body = EnergyDrain2::new(200);
        let mut runtime = runtime();

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);

        for (index, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::ParticleSpline);
            assert_eq!(state.duration, 60);
            assert_eq!(state.delta_pos2, [20.0, -10.0, 0.0]);
            assert_eq!(state.roll, (index as i32 * 90 - 90) as f32);
            assert_eq!(state.blue, 255);
            assert_eq!(state.num_segment, 7);
            // Halfway to the target: 10 units.
            assert!((state.radius - 10.0).abs() < 1e-5);
            assert!((state.speed - 10.0 / 60.0 * 2.0).abs() < 1e-5);
            // `Face(2 * position - target)` points west, i.e. render angle 270.
            assert!((state.longitude + 270.0).abs() < 1.0);
        }
    }

    #[test]
    fn energy_drain_two_rng_order_is_red_green_size_per_trail() {
        let mut body = EnergyDrain2::new(200);
        let mut runtime = runtime();
        let mut expected = EffectRuntime::new(200, [0.0, 0.0], 378);

        let mut draws = [(0i32, 0i32, 0i32); 3];
        for draw in &mut draws {
            *draw = (
                expected.rng.clamped_random(81) + 160,
                expected.rng.clamped_random(81) + 160,
                expected.rng.clamped_random(6),
            );
        }

        step(&mut body, &mut runtime);

        for (primitive, draw) in runtime.primitives.iter().zip(draws) {
            let state = &primitive.state;
            assert_eq!(state.red, draw.0);
            assert_eq!(state.green, draw.1);
            assert_eq!(state.size, draw.2 as f32 * 0.1 + 1.5);
        }
    }

    #[test]
    fn energy_drain_two_tints_the_caster_on_frames_55_to_65() {
        let mut body = EnergyDrain2::new(200);
        let mut runtime = runtime();

        runtime.frame = 54;
        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_color, None);

        runtime.frame = 55;
        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_color, Some([-1, 100, 100, 255]));
        assert_eq!(runtime.parent_flags, 1);

        runtime.begin_step();
        runtime.frame = 66;
        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_flags, 0);
    }
}
