//! `CRagEffect::SonicSlashFan` - `RagEffect.cpp:16587-16641`, plus the two
//! dispatch arms that use it:
//!
//! * `EF_ENERGYDRAIN3` (383) - `:24753-24761` calls `SonicSlashFan(0)`;
//! * `EF_ENERGYDRAIN_BLACK` (1208) - `:22263-22270` calls `SonicSlashFan(2)`.
//!
//! The body spawns three seven-segment `PP_3DPARTICLESPLINE` trails per frame
//! for frames 0..=4, fanned at `roll = -90/0/90`. Kind 0 paints
//! `(ClampedRandom(81) + 160, 255, ClampedRandom(81) + 160)`; kind 1 uses one
//! grey `ClampedRandom(55) + 200`; kind 2 one dark grey
//! `ClampedRandom(100) + 20`. Every trail then draws `ClampedRandom(6)` for
//! `size = draw * 0.1 + 1.0`.
//!
//! Both dispatch arms also tint the caster on frames 50..=80: request
//! `render.flags |= 8` and `SetColor(-1, (125 - stateCnt) * 2, (125 - stateCnt)
//! * 2, 250)`. Those writes stay in the wrappers (they belong to the dispatch,
//! not to the shared body).
//!
//! Quirk kept: `radius = lastPrim->CalcDist(this->render.position.x,
//! this->render.position.z)` measures the freshly launched primitive (seeded
//! with the effect position by `CEffectPrim::Initialize`) against the effect's
//! own position, so it is always exactly zero. The trails move only through
//! `gravSpeed`/`latiSpeed`.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `SonicSlashFan(kind)` - `RagEffect.cpp:16587-16641`.
fn sonic_slash_fan(runtime: &mut EffectRuntime, kind: i32) {
    if runtime.frame > 4 {
        return;
    }

    let angles = Angles::shared();
    let position = EffectRuntime::official_position(runtime.position);
    let mut angle = -90;

    while angle < 180 {
        // `Face(ClampedRandom(7) + render.position.x - 3.0f,
        //       ClampedRandom(7) + render.position.z - 3.0f)`.
        // Decompiled argument order: x draw first, then z draw.
        let face_x = runtime.rng.clamped_random(7) as f32 + position[0] - 3.0;
        let face_z = runtime.rng.clamped_random(7) as f32 + position[2] - 3.0;
        let face = if face_x != position[0] || face_z != position[2] {
            EffectRuntime::heading(position[0], position[2], face_x, face_z)
        } else {
            0.0
        };

        let (red, green, blue) = match kind {
            0 => (runtime.rng.clamped_random(81) + 160, 255, runtime.rng.clamped_random(81) + 160),
            1 => {
                let color = runtime.rng.clamped_random(55) + 200;
                (color, color, color)
            }
            2 => {
                let color = runtime.rng.clamped_random(100) + 20;
                (color, color, color)
            }
            _ => (0, 0, 0),
        };
        let size = runtime.rng.clamped_random(6) as f32 * 0.1 + 1.0;

        let spline = runtime.launch(PrimitiveId::ParticleSpline, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spline);

        state.duration = 60;
        state.delta_pos2 = [0.0, -10.0, 0.0];
        // Quirk: the primitive's own anchor and the effect anchor are the same
        // position, so `CalcDist` is zero.
        state.radius = 0.0;
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
        state.blue = blue;
        state.size = size;
        state.sprite = Some("이팩트\\particle1.spr");
        state.action = Some("이팩트\\particle1.act");
        state.num_segment = 7;
        state.init_segment();

        angle += 90;
    }
}

/// `EF_ENERGYDRAIN3` (383) - `CRagEffect::SonicSlashFan(0)` plus the caster
/// tint of `RagEffect.cpp:24754-24760`.
#[derive(Debug)]
pub struct EnergyDrain3 {
    lifetime: i32,
}

impl EnergyDrain3 {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for EnergyDrain3 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        sonic_slash_fan(runtime, 0);

        if (50..=80).contains(&runtime.frame) {
            runtime.add_parent_flags(8);
            let channel = ((125 - runtime.frame) * 2) as i16;
            runtime.set_parent_color(-1, channel, channel, 250);
        }

        true
    }
}

/// `EF_ENERGYDRAIN_BLACK` (1208) - `CRagEffect::SonicSlashFan(2)` plus the
/// same caster tint (`RagEffect.cpp:22263-22270`).
#[derive(Debug)]
pub struct EnergyDrainBlack {
    lifetime: i32,
}

impl EnergyDrainBlack {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for EnergyDrainBlack {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        sonic_slash_fan(runtime, 2);

        if (50..=80).contains(&runtime.frame) {
            runtime.add_parent_flags(8);
            let channel = ((125 - runtime.frame) * 2) as i16;
            runtime.set_parent_color(-1, channel, channel, 250);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    fn runtime(seed: u64) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], seed);
        runtime.position = [10.0, 0.0, 10.0];

        runtime
    }

    #[test]
    fn energy_drain_three_spawns_three_trails_on_the_first_five_frames() {
        let mut body = EnergyDrain3::new(200);
        let mut runtime = runtime(383);

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);

        for (index, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::ParticleSpline);
            assert_eq!(state.duration, 60);
            assert_eq!(state.roll, (index as i32 * 90 - 90) as f32);
            assert_eq!(state.green, 255);
            assert_eq!(state.radius, 0.0);
            assert_eq!(state.speed, 0.0);
            assert_eq!(state.num_segment, 7);
        }

        for _ in 0..4 {
            step(&mut body, &mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 15);

        // Frame 5 is outside `stateCnt > 4`.
        step(&mut body, &mut runtime);
        assert_eq!(runtime.primitives.len(), 15);
    }

    #[test]
    fn energy_drain_three_rng_order_is_face_x_face_z_red_blue_size() {
        let mut body = EnergyDrain3::new(200);
        let mut runtime = runtime(42);
        let mut expected = EffectRuntime::new(200, [0.0, 0.0], 42);
        let position = runtime.position;

        let mut draws = [(0.0f32, 0.0f32, 0i32, 0i32, 0i32); 3];
        for draw in &mut draws {
            let face_x = expected.rng.clamped_random(7) as f32 + position[0] - 3.0;
            let face_z = expected.rng.clamped_random(7) as f32 + position[2] - 3.0;
            let red = expected.rng.clamped_random(81) + 160;
            let blue = expected.rng.clamped_random(81) + 160;
            let size = expected.rng.clamped_random(6);

            *draw = (face_x, face_z, red, blue, size);
        }

        step(&mut body, &mut runtime);

        for (primitive, draw) in runtime.primitives.iter().zip(draws) {
            let state = &primitive.state;
            assert_eq!(state.red, draw.2);
            assert_eq!(state.blue, draw.3);
            assert_eq!(state.size, draw.4 as f32 * 0.1 + 1.0);

            let expected_face = if draw.0 != position[0] || draw.1 != position[2] {
                EffectRuntime::heading(position[0], position[2], draw.0, draw.1)
            } else {
                0.0
            };
            assert_eq!(state.render_angle, expected_face);
        }
    }

    #[test]
    fn energy_drain_black_uses_the_dark_grey_colour_table() {
        let mut body = EnergyDrainBlack::new(200);
        let mut runtime = runtime(1208);
        let mut expected = EffectRuntime::new(200, [0.0, 0.0], 1208);
        let position = runtime.position;
        let mut colours = [0i32; 3];

        for colour in &mut colours {
            expected.rng.clamped_random(7);
            expected.rng.clamped_random(7);
            *colour = expected.rng.clamped_random(100) + 20;
            expected.rng.clamped_random(6);
        }

        step(&mut body, &mut runtime);

        for (primitive, colour) in runtime.primitives.iter().zip(colours) {
            let state = &primitive.state;
            assert_eq!((state.red, state.green, state.blue), (colour, colour, colour));
        }

        assert_eq!(position, runtime.position);
    }

    #[test]
    fn energy_drain_three_tints_the_caster_on_frames_50_to_80() {
        let mut body = EnergyDrain3::new(200);
        let mut runtime = runtime(383);

        runtime.frame = 49;
        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_color, None);

        runtime.frame = 50;
        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_color, Some([-1, 150, 150, 250]));
        assert_eq!(runtime.parent_flags, 8);

        runtime.begin_step();
        runtime.frame = 81;
        body.on_process(&mut runtime);
        assert_eq!(runtime.parent_flags, 0);
    }
}
