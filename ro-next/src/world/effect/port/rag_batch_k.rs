//! Effects ported by the "k" parallel batch (ids whose enum variant was
//! missing).
//!
//! These are the dispatch arms that were already ported *in spirit* - the
//! shared body exists and is registered for its sibling ids - but whose
//! official id had no `EffectId` variant, so `registry.rs` could not name them.
//!
//! | effect | id | dispatch | shared body |
//! | --- | --- | --- | --- |
//! | `EF_PEONG_BLACK` | 1207 | `RagEffect.cpp:22254-22261` | `CRagEffect::SonicSlashFan(1)` (`:16587-16641`) |
//!
//! The other ids in this batch (`EF_BLOODDRAIN2` 1052, `EF_NEW_HEAVENS_DRIVE`
//! 1399, `EF_HEAL_BLUE_2` 2427, `EF_RG_COIN_BLUE` 2436, `EF_RG_COIN_BLUE_2`
//! 2437) are pure aliases of a body that is already public, so they live
//! entirely in `registry_batch_k.rs`.
//!
//! ## Why `SonicSlashFan(1)` is transliterated here
//!
//! `rag/sonic_slash_fan.rs` keeps `sonic_slash_fan` private and exposes only
//! the two fixed-kind wrappers (`EnergyDrain3` = kind 0, `EnergyDrainBlack` =
//! kind 2); that module is owned by another round. The kind-1 branch below is
//! a byte-for-byte transliteration of the same `RagEffect.cpp:16587-16641`
//! body, including the RNG order (face x draw(7), face z draw(7), grey
//! draw(55), size draw(6)) and the zero-radius `CalcDist` quirk
//! (`lastPrim->CalcDist(render.position)` measures the freshly seeded
//! primitive against its own anchor, so it is always exactly zero).

#![allow(dead_code)]

use super::angles::Angles;
use super::math::matrix_mult;
use super::primitive::PrimitiveId;
use super::runtime::{EffectRuntime, PortedEffect};

/// `SonicSlashFan(1)` - `RagEffect.cpp:16587-16641`, kind-1 branch at
/// `:16622-16626`.
fn sonic_slash_fan_kind_one(runtime: &mut EffectRuntime) {
    if runtime.frame > 4 {
        return;
    }

    let angles = Angles::shared();
    let position = runtime.position;
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

        // `kind == 1`: one grey `ClampedRandom(55) + 200`.
        let colour = runtime.rng.clamped_random(55) + 200;
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
        state.red = colour;
        state.green = colour;
        state.blue = colour;
        state.size = size;
        state.sprite = Some("이팩트\\particle1.spr");
        state.action = Some("이팩트\\particle1.act");
        state.num_segment = 7;
        state.init_segment();

        angle += 90;
    }
}

/// `EF_PEONG_BLACK` (1207) - `CRagEffect::SonicSlashFan(1)` plus the caster
/// tint of `RagEffect.cpp:22254-22261`.
///
/// Lifetime 200 (`EffectTypes.cpp:1216`).
#[derive(Debug)]
pub struct PeongBlack {
    lifetime: i32,
}

impl PeongBlack {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for PeongBlack {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        sonic_slash_fan_kind_one(runtime);

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
    fn peong_black_spawns_three_grey_trails_on_the_first_five_frames() {
        let mut body = PeongBlack::new(200);
        let mut runtime = runtime(1207);

        step(&mut body, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);

        for (index, primitive) in runtime.primitives.iter().enumerate() {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::ParticleSpline);
            assert_eq!(state.duration, 60);
            assert_eq!(state.roll, (index as i32 * 90 - 90) as f32);
            assert_eq!(state.red, state.green);
            assert_eq!(state.green, state.blue);
            assert!(state.red >= 200 && state.red < 255);
            assert_eq!(state.radius, 0.0);
            assert_eq!(state.speed, 0.0);
            assert_eq!(state.num_segment, 7);
            assert_eq!(state.sprite, Some("이팩트\\particle1.spr"));
            assert_eq!(state.action, Some("이팩트\\particle1.act"));
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
    fn peong_black_rng_order_is_face_x_face_z_grey_size() {
        let mut body = PeongBlack::new(200);
        let mut runtime = runtime(42);
        let mut expected = EffectRuntime::new(200, [0.0, 0.0], 42);
        let position = runtime.position;

        let mut draws = [(0.0f32, 0.0f32, 0i32, 0i32); 3];
        for draw in &mut draws {
            let face_x = expected.rng.clamped_random(7) as f32 + position[0] - 3.0;
            let face_z = expected.rng.clamped_random(7) as f32 + position[2] - 3.0;
            let colour = expected.rng.clamped_random(55) + 200;
            let size = expected.rng.clamped_random(6);

            *draw = (face_x, face_z, colour, size);
        }

        step(&mut body, &mut runtime);

        for (primitive, draw) in runtime.primitives.iter().zip(draws) {
            let state = &primitive.state;
            assert_eq!((state.red, state.green, state.blue), (draw.2, draw.2, draw.2));
            assert_eq!(state.size, draw.3 as f32 * 0.1 + 1.0);

            let expected_face = if draw.0 != position[0] || draw.1 != position[2] {
                EffectRuntime::heading(position[0], position[2], draw.0, draw.1)
            } else {
                0.0
            };
            assert_eq!(state.render_angle, expected_face);
        }
    }

    #[test]
    fn peong_black_tints_the_caster_on_frames_50_to_80() {
        let mut body = PeongBlack::new(200);
        let mut runtime = runtime(1207);

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

    #[test]
    fn peong_black_lifetime_is_200() {
        // `EffectTypes.cpp:1216`
        assert_eq!(PeongBlack::new(200).lifetime(), 200);
    }
}
