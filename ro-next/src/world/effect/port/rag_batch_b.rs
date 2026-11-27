//! Effects ported by the "b" parallel batch.
//!
//! This file is owned exclusively by batch "b": bodies and their tests live
//! here so parallel work never edits a shared module.

#![allow(dead_code)]

use super::angles::Angles;
use super::math::matrix_mult;
use super::primitive::PrimitiveId;
use super::runtime::{EffectRuntime, PortedEffect};

/// `g_depthAverageHalf` - `MapSkyMgr.cpp:27`.
const DEPTH_AVERAGE_HALF: f32 = 0.5;

// ---------------------------------------------------------------------------
// CRagEffect::ParticleTime
// ---------------------------------------------------------------------------

/// `CRagEffect::ParticleTime(const char* name, char kind)` -
/// `RagEffect.cpp:12121-12177`.
///
/// `EF_SMA_READY` (kind 2): a prim 59 (`PrimParticleUp`) orb burst every four
/// frames between frames 40 and 120, carrying the `pok1`/`pok3` texture pair.
///
/// The original never reads its `name` argument (the two textures are
/// hard-coded); the field is kept so the call keeps its official shape.
#[derive(Debug)]
pub struct ParticleTime {
    name: &'static str,
    kind: i32,
    lifetime: i32,
}

impl ParticleTime {
    pub fn new(name: &'static str, kind: i32, lifetime: i32) -> Self {
        Self { name, kind, lifetime }
    }
}

impl PortedEffect for ParticleTime {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let kind = self.kind;

        // `this->render.position = this->state.parent->render.position` - the
        // runtime already anchors the effect at the parent position.

        let min_count = if kind != 2 { 0 } else { 40 };
        if frame < min_count {
            return true;
        }

        let max_count = if kind != 2 { 80 } else { 120 };
        if frame > max_count {
            return true;
        }

        if frame % 4 != 0 {
            return true;
        }

        let lifetime = runtime.lifetime;
        let position = runtime.position;

        // Per-slot RNG order: distance (10), x (11), z (11), rise speed (20).
        let mut distances = [0.0f32; 4];
        let mut xs = [0.0f32; 4];
        let mut zs = [0.0f32; 4];
        let mut speeds = [0.0f32; 4];

        for index in 0..4usize {
            distances[index] = runtime.rng.clamped_random(10) as f32 * 0.1 + 2.0;
            xs[index] = runtime.rng.clamped_random(11) as f32 + position[0] - 5.0;
            zs[index] = runtime.rng.clamped_random(11) as f32 + position[2] - 5.0;
            speeds[index] = runtime.rng.clamped_random(20) as f32 * 0.01 + 0.1;
        }

        let orbs = runtime.launch(PrimitiveId::ParticleUp, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(orbs);

        state.render_flag = 1 | 4; // RSF_ALPHA | RSF_CUSTOM_BLEND
        state.duration = lifetime;
        state.total_texture = 2;
        state.texture = Some("effect\\pok1.tga");
        state.texture_secondary = Some("effect\\pok3.tga");
        state.size = 0.0;
        state.position = position;

        for index in 0..4usize {
            let slot = &mut state.slots[index];

            slot.life = 1;
            slot.process = 0;
            slot.distance = distances[index];
            slot.height[8] = 0.0;
            slot.alpha_b = 0;
            slot.vec_b_now[0] = xs[index];
            slot.vec_b_now[2] = zs[index];
            slot.vec_b_now[1] = position[1] - 6.0;
            slot.rot_start = 0;
            slot.height[1] = 0.0;
            slot.height[2] = speeds[index];
            slot.full_display_angle = 0;
            slot.flag1[0] = 0;
            slot.flag1[1] = 1;
            slot.flag1[2] = match kind {
                0 => 11,
                1 => 1,
                2 => 1,
                3 => 13,
                _ => 0,
            };
            slot.height[7] = 0.0;
        }

        true
    }
}

// ---------------------------------------------------------------------------
// CRagEffect::StoneWall
// ---------------------------------------------------------------------------

/// `CRagEffect::StoneWall()` - `RagEffect.cpp:13088-13124`.
///
/// `EF_EARTHWALL`: three `PP_3DQUADHORN` stone slabs dropped thirty units
/// straight up, each with its own random heading. The body rewrites the
/// effect's own duration to 9999 (`state.duration = 9999`).
#[derive(Debug)]
pub struct StoneWall {
    lifetime: i32,
}

impl StoneWall {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for StoneWall {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();

        runtime.play_sound("effect\\wizard_icewall.wav");

        // C++ `this->state.duration = 9999;` overrides the table lifetime.
        runtime.lifetime = 9999;
        let duration = runtime.lifetime;

        // `for (offset = -2; offset < 4; offset += 2)`
        for offset in (-2..4).step_by(2) {
            // RNG order: heading (360), size (40), height (100), longitude
            // (360), latitude (6).
            let angle = runtime.rng.clamped_random(360);
            let sine = angles.sine(angle);
            let cosine = angles.cosine(angle);
            let size = (runtime.rng.clamped_random(40) / 100) as f32 + 2.5;
            let height = (runtime.rng.clamped_random(100) + 300) as f32 / 10.0;
            let longitude = runtime.rng.clamped_random(360) as f32;
            let latitude = (runtime.rng.clamped_random(6) + 87) as f32;

            let slab = runtime.launch(PrimitiveId::QuadHorn3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(slab);

            state.duration = duration;
            state.pattern |= 1;
            state.delta_pos2 = [sine * 0.0, 30.0, -(offset as f32) * cosine];
            state.size = size;
            state.height_size = height;
            state.longitude = longitude;
            state.latitude = latitude;
            state.ch_cnt = 20;
            state.alpha = 200.0;
            state.transform.make_x_rotation(angles.radian(latitude as i32));
            state.transform.append_y_rotation(angles.radian(longitude as i32));
            state.rot_transform = state.transform;
            state.speed = 3.0;
            let decay = -(state.speed / 40.0);
            state.accel = decay + decay;
            state.fade_out_cnt = state.duration - 10;
            state.texture = Some("effect\\stone.bmp");
        }

        true
    }
}

// ---------------------------------------------------------------------------
// CRagEffect::Hit3
// ---------------------------------------------------------------------------

/// `CRagEffect::Hit3()` - `RagEffect.cpp:863-935`.
///
/// `EF_HIT3`: two ground-facing `PP_3DCYLINDER` lens rings plus eight
/// `PP_3DPARTICLE` sparks fanned over the caster's facing.
#[derive(Debug)]
pub struct Hit3 {
    lifetime: i32,
}

impl Hit3 {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for Hit3 {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();

        runtime.play_sound("effect\\EF_hit3.wav");

        // `int facing = static_cast<int>(180.0f - parent.render.angle);`
        let mut facing = (180.0f32 - runtime.parent_angle) as i32;
        if facing < 0 {
            facing += 360;
        }
        let facing_degrees = facing as f32;

        // Ring 1: narrow `lens2` cylinder.
        let ring = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.duration = 15;
        state.longitude = facing_degrees;
        state.latitude = -90.0;
        state.transform.make_x_rotation(angles.radian(-90));
        state.transform.append_y_rotation(angles.radian(facing));
        state.delta_pos2 = [0.0, -10.0, 0.0];
        state.speed = 0.7;
        state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
        state.outer_size = 1.5;
        state.inner_size = 1.5;
        state.height_speed = 0.5;
        state.height_accel = 0.2;
        state.fade_out_cnt = state.duration - state.duration / 3;
        state.texture = Some("effect\\lens2.tga");

        // Ring 2: wide `lens2` cylinder.
        let ring = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.duration = 15;
        state.longitude = facing_degrees;
        state.latitude = -90.0;
        state.transform.make_x_rotation(angles.radian(-90));
        state.transform.append_y_rotation(angles.radian(facing));
        state.delta_pos2 = [0.0, -10.0, 0.0];
        state.speed = 0.7;
        state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
        state.outer_size = 4.0;
        state.inner_size = 1.5;
        state.height_speed = 0.25;
        state.height_accel = 0.2;
        state.fade_out_cnt = state.duration - state.duration / 3;
        state.texture = Some("effect\\lens2.tga");

        // Eight sparks. RNG order: longitude (80), latitude (100), travel (4),
        // speed (120), duration (24), size (100).
        for _ in 0..8 {
            let longitude = (runtime.rng.clamped_random(80) + (facing - 40)) as f32;
            let latitude = (runtime.rng.clamped_random(100) - 50) as f32;
            let travel = (runtime.rng.clamped_random(4) + 4) as f32;
            let speed = (runtime.rng.clamped_random(120) + 80) as f32 / 100.0;
            let duration = runtime.rng.clamped_random(24) + 6;
            let size = (runtime.rng.clamped_random(100) + 60) as f32 / 100.0;

            let spark = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spark);

            state.longitude = longitude;
            state.latitude = latitude;
            state.transform.make_y_rotation(angles.radian(longitude as i32));
            state.transform.append_x_rotation(angles.radian(latitude as i32));

            let placed = matrix_mult([0.0, 0.0, travel], &state.transform.values);
            state.delta_pos3 = placed;
            state.delta_pos2 = [placed[0] + 0.0, placed[1] - 10.0, placed[2] + 0.0];

            state.speed = speed;
            state.duration = duration;
            state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
            state.size = size;
            state.size_speed = -(state.size / state.duration as f32);
            state.fade_out_cnt = 0;
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.num_segment = 3;
            state.init_segment();
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRng, EffectRuntime, step};

    // -- ParticleTime -------------------------------------------------------

    #[test]
    fn particle_time_waits_for_frame_forty() {
        let mut effect = ParticleTime::new("effect\\white02.bmp", 2, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 641);

        while runtime.frame < 40 {
            assert!(step(&mut effect, &mut runtime));
            assert!(runtime.primitives.is_empty(), "nothing before frame 40");
        }

        step(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);
    }

    #[test]
    fn particle_time_bursts_every_four_frames_from_40_to_120() {
        let mut effect = ParticleTime::new("effect\\white02.bmp", 2, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 641);

        for _ in 0..200 {
            if !step(&mut effect, &mut runtime) {
                break;
            }
        }

        // Frames 40, 44, ..., 120: (120 - 40) / 4 + 1 = 21 bursts.
        assert_eq!(runtime.primitives.len(), 21);
    }

    #[test]
    fn particle_time_kind_two_uses_the_official_orb_values() {
        let mut effect = ParticleTime::new("effect\\white02.bmp", 2, 200);
        let mut runtime = EffectRuntime::new(200, [0.0, 0.0], 641);

        while runtime.frame < 40 {
            step(&mut effect, &mut runtime);
        }
        step(&mut effect, &mut runtime);

        let orbs = &runtime.primitives[0].state;
        assert_eq!(orbs.count, PrimitiveId::ParticleUp);
        assert_eq!(orbs.render_flag, 1 | 4);
        assert_eq!(orbs.duration, 200);
        assert_eq!(orbs.total_texture, 2);
        assert_eq!(orbs.texture, Some("effect\\pok1.tga"));
        assert_eq!(orbs.texture_secondary, Some("effect\\pok3.tga"));
        assert_eq!(orbs.size, 0.0);

        for slot in &orbs.slots {
            assert_eq!(slot.life, 1);
            assert_eq!(slot.flag1[0], 0);
            assert_eq!(slot.flag1[1], 1);
            // kind 2 -> flag1[2] == 1
            assert_eq!(slot.flag1[2], 1);
            assert!((2.0..3.0).contains(&slot.distance), "distance {}", slot.distance);
            // Spawned at -6.0, then the first primitive pass moves it up.
            assert_eq!(slot.vec_b_now[1], -6.0 - slot.height[2]);
            assert_eq!(slot.height[8], 0.0);
            assert!((0.1..0.3).contains(&slot.height[2]));
        }
    }

    #[test]
    fn particle_time_kind_three_uses_the_thirteen_flag() {
        let mut effect = ParticleTime::new("effect\\white02.bmp", 3, 300);
        let mut runtime = EffectRuntime::new(300, [0.0, 0.0], 641);

        for _ in 0..80 {
            step(&mut effect, &mut runtime);
            if !runtime.primitives.is_empty() {
                break;
            }
        }

        let orbs = &runtime.primitives[0].state;
        assert!(orbs.slots.iter().all(|slot| slot.flag1[2] == 13));
    }

    // -- StoneWall ----------------------------------------------------------

    #[test]
    fn stone_wall_drops_three_slabs_and_extends_its_duration() {
        let mut effect = StoneWall::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 521);

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\wizard_icewall.wav".to_owned()]);
        assert_eq!(runtime.lifetime, 9999, "state.duration = 9999");
        assert_eq!(runtime.primitives.len(), 3, "offsets -2, 0, 2");

        for slab in &runtime.primitives {
            let state = &slab.state;
            assert_eq!(state.count, PrimitiveId::QuadHorn3d);
            assert_eq!(state.duration, 9999);
            assert_eq!(state.pattern, 1);
            // QUIRK: integer division `ClampedRandom(40) / 100` is always 0.
            assert_eq!(state.size, 2.5);
            assert_eq!(state.delta_pos2[0], 0.0, "sine * 0.0f");
            assert_eq!(state.delta_pos2[1], 30.0);
            assert_eq!(state.ch_cnt, 20);
            assert_eq!(state.fade_out_cnt, 9989);
            assert_eq!(state.texture, Some("effect\\stone.bmp"));
            assert!(state.delta_pos2[2].abs() <= 2.0);
            assert!((30.0..40.0).contains(&state.height_size));
            assert!((87.0..93.0).contains(&state.latitude));
        }

        // Middle slab: offset 0 -> zero drift.
        assert_eq!(runtime.primitives[1].state.delta_pos2[2], 0.0);
    }

    #[test]
    fn stone_wall_uses_the_official_drift_for_the_first_slab() {
        let seed = 521;
        let mut effect = StoneWall::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], seed);

        step(&mut effect, &mut runtime);

        // Replay the first slab's RNG: heading (360), size (40), height (100),
        // longitude (360), latitude (6).
        let mut rng = EffectRng::new(seed);
        let angle = rng.clamped_random(360);
        let _size = rng.clamped_random(40);
        let _height = rng.clamped_random(100);
        let _longitude = rng.clamped_random(360);
        let _latitude = rng.clamped_random(6);

        let cosine = Angles::shared().cosine(angle);
        let expected = -(-2.0f32) * cosine;
        let drift = runtime.primitives[0].state.delta_pos2[2];
        assert!((drift - expected).abs() < 1e-6, "{drift} vs {expected}");
    }

    // -- Hit3 ---------------------------------------------------------------

    #[test]
    fn hit3_spawns_two_rings_and_eight_sparks() {
        let mut effect = Hit3::new(50);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 333);
        runtime.parent_angle = 90.0;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\EF_hit3.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 10);

        let narrow = &runtime.primitives[0].state;
        assert_eq!(narrow.count, PrimitiveId::Cylinder3d);
        assert_eq!(narrow.duration, 15);
        assert_eq!(narrow.longitude, 90.0, "180 - parent angle");
        assert_eq!(narrow.latitude, -90.0);
        assert_eq!(narrow.delta_pos2, [0.0, -10.0, 0.0]);
        assert_eq!(narrow.outer_size, 1.5);
        assert_eq!(narrow.inner_size, 1.5);
        assert_eq!(narrow.height_accel, 0.2);
        assert_eq!(narrow.fade_out_cnt, 10);
        assert!((narrow.accel + (0.7 / 15.0) * DEPTH_AVERAGE_HALF).abs() < 1e-6);
        assert_eq!(narrow.texture, Some("effect\\lens2.tga"));

        let wide = &runtime.primitives[1].state;
        assert_eq!(wide.outer_size, 4.0);
        assert_eq!(wide.inner_size, 1.5);
        assert_eq!(wide.height_accel, 0.2);

        for spark in &runtime.primitives[2..] {
            let state = &spark.state;
            assert_eq!(state.count, PrimitiveId::Particle3d);
            assert_eq!(state.num_segment, 3);
            assert_eq!(state.records.len(), 3);
            assert_eq!(state.sprite, Some("이팩트\\particle1.spr"));
            assert_eq!(state.action, Some("이팩트\\particle1.act"));
            assert_eq!(state.fade_out_cnt, 0);
            assert!((6..=29).contains(&state.duration));
            assert!((0.6..=1.59).contains(&state.size));
            assert!((0.8..=1.99).contains(&state.speed));
            // Sparks fan over facing - 40 .. facing + 39.
            assert!((50.0..=129.0).contains(&state.longitude), "{}", state.longitude);
            assert!((-50.0..=49.0).contains(&state.latitude));
        }
    }

    #[test]
    fn hit3_facing_wraps_negative_angles() {
        let mut effect = Hit3::new(50);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 333);
        runtime.parent_angle = 270.0;

        step(&mut effect, &mut runtime);

        // 180 - 270 = -90, wrapped to 270.
        assert_eq!(runtime.primitives[0].state.longitude, 270.0);
    }

    #[test]
    fn hit3_replays_the_official_rng_order_for_the_first_spark() {
        let seed = 333;
        let mut effect = Hit3::new(50);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], seed);
        runtime.parent_angle = 90.0;

        step(&mut effect, &mut runtime);

        let mut rng = EffectRng::new(seed);
        let longitude = (rng.clamped_random(80) + (90 - 40)) as f32;
        let latitude = (rng.clamped_random(100) - 50) as f32;
        let travel = (rng.clamped_random(4) + 4) as f32;
        let speed = (rng.clamped_random(120) + 80) as f32 / 100.0;
        let duration = rng.clamped_random(24) + 6;
        let size = (rng.clamped_random(100) + 60) as f32 / 100.0;

        let spark = &runtime.primitives[2].state;
        assert_eq!(spark.longitude, longitude);
        assert_eq!(spark.latitude, latitude);
        // Spawned at `speed`, then the first particle pass adds `accel`.
        let accel = -(speed / duration as f32) * DEPTH_AVERAGE_HALF;
        assert!((spark.speed - (speed + accel)).abs() < 1e-6);
        assert_eq!(spark.duration, duration);
        assert!((spark.size_speed + size / duration as f32).abs() < 1e-6);

        let angles = Angles::shared();
        let mut transform = super::super::math::Matrix::identity();
        transform.make_y_rotation(angles.radian(longitude as i32));
        transform.append_x_rotation(angles.radian(latitude as i32));
        let placed = matrix_mult([0.0, 0.0, travel], &transform.values);

        assert_eq!(spark.delta_pos3, placed);
        assert_eq!(spark.delta_pos2, [placed[0] + 0.0, placed[1] - 10.0, placed[2] + 0.0]);
    }
}
