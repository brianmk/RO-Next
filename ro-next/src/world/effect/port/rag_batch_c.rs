//! Effects ported by the "c" parallel batch.
//!
//! This file is owned exclusively by batch "c": bodies and their tests live
//! here so parallel work never edits a shared module.

#![allow(dead_code)]

use super::angles::Angles;
use super::math::matrix_mult;
use super::primitive::PrimitiveId;
use super::runtime::{EffectRuntime, PortedEffect};

/// `g_depthAverageHalf` - `MapSkyMgr.cpp:27`.
const DEPTH_AVERAGE_HALF: f32 = 0.5;

/// `CRenderObject::Face(x, z)` - `ActorGeometry.cpp:47-64`.
///
/// The effect bodies call this on a freshly launched primitive, whose
/// `render.position` the constructor copies from the master
/// (`EffectPrimitiveState.cpp:72-88`), so the port passes the effect anchor in.
fn prim_face(position: [f32; 3], x: f32, z: f32) -> f32 {
    let delta_x = x - position[0];
    let delta_z = z - position[2];
    let mut angle = 0.0f32;

    if delta_x != 0.0 || delta_z != 0.0 {
        angle = ((delta_x as f64).atan2((-delta_z) as f64) as f32 / 3.141592f32) * 180.0;

        if angle >= 360.0 {
            angle -= 360.0;
        }

        if angle < 0.0 {
            angle += 360.0;
        }
    }

    angle
}

// ---------------------------------------------------------------------------
// CRagEffect::Yufitel
// ---------------------------------------------------------------------------

/// `CRagEffect::Yufitel(char F1)` - `RagEffect.cpp:1224-1293`.
///
/// `EF_YUFITEL`: a `PP_3DTEXTURE` (15) `thunder_center` quad facing the target
/// plus a six-frame `thunder_ball_a..f` quad on the same frame.
#[derive(Debug)]
pub struct Yufitel {
    kind: i32,
    lifetime: i32,
}

impl Yufitel {
    pub fn new(kind: i32, lifetime: i32) -> Self {
        Self { kind, lifetime }
    }
}

impl PortedEffect for Yufitel {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();
        let target = runtime.target_offset;
        let position = runtime.position;

        runtime.play_sound("effect\\hunter_shockwavetrap.wav");

        // `lastPrim->Face(state.deltaPos.x, state.deltaPos.z)` and the matching
        // `lastPrim->CalcDist(...)` both run from the master's anchor.
        let face = prim_face(position, target[0], target[2]);
        let distance = runtime.calc_dist(target[0], target[2]);

        // First quad: `thunder_center` / `twirl_soft`, 3.5 wide.
        let first = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(first);

        state.duration = 20;
        state.pattern |= 0x40u32;
        state.pattern |= 0x400u32;
        state.render_flag |= 2;
        state.width_size = 3.5;
        state.height_size = 3.5;
        state.render_angle = face;
        state.longitude = -state.render_angle;
        state.render_angle = 180.0 - state.render_angle;

        if state.render_angle < 0.0 {
            state.render_angle += 360.0;
        }

        state.speed = -(distance / state.duration as f32);
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));
        state.delta_pos2[1] = -5.0;
        state.alpha = 170.0;
        state.texture = Some(if self.kind == 0 {
            "effect\\thunder_center.bmp"
        } else {
            "effect\\twirl_soft.bmp"
        });

        // Second quad: six animated `thunder_ball_*` frames, 4.5 wide.
        let second = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(second);

        state.duration = 20;
        state.pattern = 0x40u32;
        state.pattern |= 0x400u32;
        state.render_flag |= 2;
        state.repeat_anim = 1;
        state.width_size = 4.5;
        state.height_size = 4.5;
        state.render_angle = face;
        state.longitude = -state.render_angle;
        state.render_angle = 180.0 - state.render_angle;

        if state.render_angle < 0.0 {
            state.render_angle += 360.0;
        }

        state.speed = -(distance / state.duration as f32);
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));
        state.delta_pos2[1] = -5.0;
        state.anim_speed = 1;
        state.total_texture = 6;
        state.frame_count = 6;
        state.texture_frames = if self.kind == 0 {
            vec![
                "effect\\thunder_ball_a.bmp",
                "effect\\thunder_ball_b.bmp",
                "effect\\thunder_ball_c.bmp",
                "effect\\thunder_ball_d.bmp",
                "effect\\thunder_ball_e.bmp",
                "effect\\thunder_ball_f.bmp",
            ]
        } else {
            vec![
                "effect\\twirl_soft.bmp",
                "effect\\thunder_ball_b.bmp",
                "effect\\twirl_soft.bmp",
                "effect\\thunder_ball_d.bmp",
                "effect\\twirl_soft.bmp",
                "effect\\thunder_ball_f.bmp",
            ]
        };
        state.texture = state.texture_frames.first().copied();

        true
    }
}

// ---------------------------------------------------------------------------
// CRagEffect::YufitelHit
// ---------------------------------------------------------------------------

/// `CRagEffect::YufitelHit(char kind)` - `RagEffect.cpp:11076-11134`.
///
/// `EF_YUFITELHIT` (kind 0) and `EF_YUFITEL2` (kind 1): a small
/// `PP_3DTEXTURE` (15) hit flash on every twentieth frame, plus the long
/// five-frame plasma blast spawned once on frame 10.
#[derive(Debug)]
pub struct YufitelHit {
    kind: i32,
    lifetime: i32,
}

impl YufitelHit {
    pub fn new(kind: i32, lifetime: i32) -> Self {
        Self { kind, lifetime }
    }
}

impl PortedEffect for YufitelHit {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame % 20 == 0 {
            let flash = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(flash);

            state.duration = 10;
            state.pattern |= 0x40u32;
            state.render_flag |= 2;
            state.render_flag |= 0x200;
            state.delta_pos2[1] = -5.0;
            state.width_speed = 2.5;
            state.height_speed = 2.5;
            state.change_point = 10;
            state.change_width_speed = 0.0;
            state.change_height_speed = 0.0;
            state.change_width_accel = -state.change_width_speed / 50.0;
            state.change_height_accel = -state.change_height_speed / 50.0;
            state.fade_out_cnt = 1;
            state.texture = Some(if self.kind == 0 {
                "effect\\thunder_pang.bmp"
            } else {
                "effect\\pokjuk_d.bmp"
            });
        }

        if frame != 10 {
            return true;
        }

        let blast = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(blast);

        state.duration = 300;
        state.pattern |= 0x40u32;
        state.render_flag |= 0x200;
        state.render_flag |= 2;
        state.delta_pos2[1] = -5.0;
        state.repeat_anim = 1;
        state.width_size = 7.5;
        state.height_size = 7.5;
        state.anim_speed = 1;
        state.total_texture = 5;
        state.frame_count = 5;
        state.texture_frames = if self.kind == 0 {
            vec![
                "effect\\thunder_plazma_blast_a.bmp",
                "effect\\thunder_plazma_blast_b.bmp",
                "effect\\thunder_ball_d.bmp",
                "effect\\thunder_ball_e.bmp",
                "effect\\thunder_ball_f.bmp",
            ]
        } else {
            vec![
                "effect\\twirl_soft.bmp",
                "effect\\thunder_ball_b.bmp",
                "effect\\twirl_soft.bmp",
                "effect\\thunder_ball_c.bmp",
                "effect\\twirl_soft.bmp",
            ]
        };
        state.texture = state.texture_frames.first().copied();

        true
    }
}

// ---------------------------------------------------------------------------
// CRagEffect::HitDark
// ---------------------------------------------------------------------------

/// `CRagEffect::HitDark()` - `RagEffect.cpp:12390-12468`.
///
/// `EF_HITDARK`: a ground-facing `PP_3DCYLINDER` ring plus two
/// `PP_3DPARTICLE` and two `PP_3DPARTICLEGRAVITY` sparks, all tinted 20/20/20.
#[derive(Debug)]
pub struct HitDark {
    lifetime: i32,
}

impl HitDark {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for HitDark {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();
        let mut facing = (180.0f32 - runtime.parent_angle) as i32;

        if facing < 0 {
            facing += 360;
        }

        // Ring.
        let ring = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(ring);

        state.duration = 10;
        state.longitude = facing as f32;
        state.latitude = -90.0;
        state.transform.make_x_rotation(angles.radian(state.latitude as i32));
        state.transform.append_y_rotation(angles.radian(state.longitude as i32));
        state.delta_pos2 = [0.0, -10.0, 0.0];
        state.speed = 0.7;
        state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
        state.outer_size = 10.0;
        state.inner_size = 5.0;
        state.height_size = 3.5;
        state.fade_out_cnt = state.duration - state.duration / 2;
        state.texture = Some("effect\\ring_blue.tga");

        // Two sprite sparks. RNG order: longitude (80), latitude (100), travel
        // (4), speed (90), duration (24), size (100).
        for _ in 0..2 {
            let longitude = (runtime.rng.clamped_random(80) + (facing - 40)) as f32;
            let latitude = (runtime.rng.clamped_random(100) - 50) as f32;
            let travel = (runtime.rng.clamped_random(4) + 4) as f32;
            let speed = (runtime.rng.clamped_random(90) + 60) as f32 / 100.0;
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
            state.red = 20;
            state.green = 20;
            state.blue = 20;
            state.fade_out_cnt = 0;
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.num_segment = 3;
            state.init_segment();
        }

        // Two gravity sparks. RNG order: longitude (80), latitude (100),
        // travel (4), duration (24), speed (90), gravity (90), size (100).
        for _ in 0..2 {
            let longitude = (runtime.rng.clamped_random(80) + (facing + 140)) as f32;
            let latitude = (runtime.rng.clamped_random(100) - 50) as f32;
            let travel = (runtime.rng.clamped_random(4) + 4) as f32;
            let duration = runtime.rng.clamped_random(24) + 6;
            let speed = (runtime.rng.clamped_random(90) + 60) as f32 / 100.0;
            let gravity = -(runtime.rng.clamped_random(90) + 30) as f32 / 100.0;
            let size = (runtime.rng.clamped_random(100) + 60) as f32 / 100.0;

            let spark = runtime.launch(PrimitiveId::Particle3dGravity, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(spark);

            state.longitude = longitude;
            state.latitude = latitude;
            state.transform.make_y_rotation(angles.radian(longitude as i32));
            state.transform.append_x_rotation(angles.radian(latitude as i32));

            let placed = matrix_mult([0.0, 0.0, travel], &state.transform.values);
            state.delta_pos3 = placed;
            state.delta_pos2 = [placed[0] + 0.0, placed[1] - 10.0, placed[2] + 0.0];
            state.transform.make_y_rotation(angles.radian(longitude as i32));

            state.duration = duration;
            state.speed = speed;
            state.accel = -(state.speed / state.duration as f32) * DEPTH_AVERAGE_HALF;
            state.grav_speed = gravity;
            state.grav_accel = -(state.grav_speed / state.duration as f32 * 2.0);
            state.size = size;
            state.red = 20;
            state.green = 20;
            state.blue = 20;
            state.fade_out_cnt = 0;
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.num_segment = 3;
            state.init_segment();
        }

        true
    }
}

// ---------------------------------------------------------------------------
// CRagEffect::HealSP
// ---------------------------------------------------------------------------

/// `CRagEffect::HealSP()` - `RagEffect.cpp:12897-12972`.
///
/// `EF_HEALSP`: three blue `PP_3DCYLINDER` columns on frame 0 plus a
/// `PP_3DPARTICLEORBIT` (7) mote on every even frame. The body rewrites the
/// effect duration to 100 (`state.duration = 100`), although the lifetime
/// table says 50.
#[derive(Debug)]
pub struct HealSp {
    lifetime: i32,
}

impl HealSp {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for HealSp {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        // `this->render.position = this->state.parent->render.position` - the
        // runtime already anchors the effect at the parent position.

        if runtime.frame == 0 {
            // `this->state.duration = 100;` - the body overrides the table
            // lifetime, and every column below copies it.
            runtime.lifetime = 100;
            let lifetime = runtime.lifetime;

            for (outer_size, height_size, max_alpha) in [(2.5f32, 15.0f32, 16.0f32), (3.5, 25.0, 32.0), (4.5, 35.0, 64.0)] {
                let ring = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(ring);

                state.duration = lifetime;
                state.long_speed = 5.0;
                state.outer_size = outer_size;
                state.inner_size = outer_size;
                state.height_size = height_size;
                state.red = 0x20;
                state.green = 0xB0;
                state.blue = 0xE8;
                state.alpha = 0.0;
                state.max_alpha = max_alpha;
                state.alpha_speed = state.max_alpha / 30.0;
                state.fade_out_cnt = state.duration - 20;
                state.texture = Some("effect\\alpha_down.tga");
            }

            runtime.play_sound("_heal_effect.wav");
        }

        if runtime.frame & 1 != 0 {
            return true;
        }

        // Bare `rand();` in the original consumes one value before the radius.
        runtime.rng.discard();
        let longitude = runtime.rng.clamped_random(360) as f32;

        let angles = Angles::shared();
        let orbit = runtime.launch(PrimitiveId::Orbit, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(orbit);

        state.radius = 3.0;
        state.duration = 30;
        state.longitude = longitude;
        state.grav_speed = -1.2;
        state.grav_accel = -(state.grav_speed / state.duration as f32) / 1.5;
        state.transform.make_y_rotation(angles.radian(state.longitude as i32));

        let forward = [0.0f32, 0.0, state.radius];
        state.delta_pos2 = matrix_mult(forward, &state.transform.values);
        state.size = 0.55;
        state.fade_out_cnt = state.duration - state.duration / 3;
        state.sprite = Some("이팩트\\particle2.spr");
        state.action = Some("이팩트\\particle2.act");
        state.init_segment();

        true
    }
}

// ---------------------------------------------------------------------------
// CRagEffect::WarpZone
// ---------------------------------------------------------------------------

/// `CRagEffect::WarpZone()` - `RagEffect.cpp:12707-12772`.
///
/// `EF_WARPZONE`: the big `alpha_down` ground disc, a `ring_blue` pulse every
/// 28 frames, and an orbit mote every 10 frames. The body resets its own
/// `stateCnt` to 0 on frame 145, restarting the disc.
#[derive(Debug)]
pub struct WarpZone {
    lifetime: i32,
}

impl WarpZone {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for WarpZone {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let angles = Angles::shared();

        // `if (stateCnt == 145) stateCnt = 0; else if (stateCnt != 0) goto
        // check_cadence;` - the reset falls through into the disc spawn, and
        // the cadence checks below read the reset counter.
        let spawn_disc = if runtime.frame == 145 {
            runtime.frame = 0;
            true
        } else {
            runtime.frame == 0
        };

        if spawn_disc {
            // `g_modeMgr.GetGameMode();` is a no-op in the original body.

            let disc = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(disc);

            state.duration = 158;
            state.pattern |= 1;
            state.radius = 15.0;
            state.delta_pos2[1] -= 1.0;
            state.latitude = 90.0;
            state.alpha = 0.0;
            state.max_alpha = 128.0;
            state.alpha_speed = state.max_alpha / 10.0;
            state.fade_out_cnt = state.duration - 10;
            state.init_segment();
            state.texture = Some("effect\\alpha_down.tga");
        }

        let frame = runtime.frame;

        if frame % 28 == 0 && frame < 117 {
            let ring = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(ring);

            state.duration = 80;
            state.inner_size = 4.0;
            state.latitude = 90.0;
            state.delta_pos2[1] -= 1.0;
            state.transform.make_x_rotation(angles.radian(state.latitude as i32));
            state.radius = 14.0;
            state.radius_speed = -0.15;
            state.alpha = 5.0;
            state.max_alpha = 200.0;
            state.alpha_speed = 10.0;
            state.ch_point.push(20.0);
            state.ch_val1.push(0.0);
            state.fade_out_cnt = state.duration - state.duration / 5;
            state.init_segment();
            state.texture = Some("effect\\ring_blue.tga");
        }

        if frame % 10 == 0 {
            // `ClampedRandom(200) / 100` and `ClampedRandom(10) / 10` are
            // integer divisions in the original.
            let radius = (runtime.rng.clamped_random(200) / 100) as f32 * 0.75 + 5.0;
            let longitude = runtime.rng.clamped_random(360) as f32;
            // Bare `rand();` between the heading and the spin speed.
            runtime.rng.discard();
            let size = (runtime.rng.clamped_random(10) / 10) as f32 + 0.7;

            let mote = runtime.launch(PrimitiveId::Orbit, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(mote);

            state.radius = radius;
            state.radius_speed = 0.0005;
            state.duration = 130;
            state.longitude = longitude;
            state.long_speed = 2.0;
            state.grav_speed = -0.2;
            state.grav_accel = -(state.grav_speed / state.duration as f32) / 1.5;
            state.transform.make_y_rotation(angles.radian(state.longitude as i32));

            let forward = [0.0f32, 0.0, state.radius];
            state.delta_pos2 = matrix_mult(forward, &state.transform.values);
            state.size = size;
            state.fade_out_cnt = state.duration - state.duration / 3;
            state.sprite = Some("이팩트\\particle1.spr");
            state.action = Some("이팩트\\particle1.act");
            state.init_segment();
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::math::Matrix;
    use crate::world::effect::port::runtime::{EffectRng, step};

    /// One body step without the primitive pass, so the assertions see the
    /// exact values the body writes (the primitive motion would integrate
    /// them).
    fn run_body(effect: &mut dyn PortedEffect, runtime: &mut EffectRuntime) -> bool {
        effect.on_process(runtime)
    }

    // -- Yufitel ------------------------------------------------------------

    fn yufitel_runtime(seed: u64) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], seed);
        runtime.target_offset = [10.0, 0.0, 0.0];
        runtime
    }

    #[test]
    fn yufitel_spawns_the_two_thunder_quads_on_frame_zero() {
        let mut effect = Yufitel::new(0, 250);
        let mut runtime = yufitel_runtime(71);

        assert!(run_body(&mut effect, &mut runtime));

        assert_eq!(runtime.sounds, vec!["effect\\hunter_shockwavetrap.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 2);

        let first = &runtime.primitives[0].state;
        assert_eq!(first.count, PrimitiveId::Texture3d);
        assert_eq!(first.duration, 20);
        assert_eq!(first.pattern, 0x40 | 0x400);
        assert_eq!(first.render_flag, 1 | 2);
        assert_eq!(first.width_size, 3.5);
        assert_eq!(first.height_size, 3.5);
        assert_eq!(first.delta_pos2[1], -5.0);
        assert_eq!(first.alpha, 170.0);
        assert_eq!(first.total_texture, 1);
        assert_eq!(first.texture, Some("effect\\thunder_center.bmp"));

        let second = &runtime.primitives[1].state;
        assert_eq!(second.count, PrimitiveId::Texture3d);
        assert_eq!(second.duration, 20);
        assert_eq!(second.pattern, 0x40 | 0x400);
        assert_eq!(second.render_flag, 1 | 2);
        assert_eq!(second.repeat_anim, 1);
        assert_eq!(second.width_size, 4.5);
        assert_eq!(second.height_size, 4.5);
        assert_eq!(second.delta_pos2[1], -5.0);
        assert_eq!(second.anim_speed, 1);
        assert_eq!(second.total_texture, 6);
        assert_eq!(second.frame_count, 6);
        assert_eq!(second.texture_frames, vec![
            "effect\\thunder_ball_a.bmp",
            "effect\\thunder_ball_b.bmp",
            "effect\\thunder_ball_c.bmp",
            "effect\\thunder_ball_d.bmp",
            "effect\\thunder_ball_e.bmp",
            "effect\\thunder_ball_f.bmp",
        ]);
        assert_eq!(second.texture, Some("effect\\thunder_ball_a.bmp"));
    }

    #[test]
    fn yufitel_faces_the_target_and_reports_the_official_heading() {
        let mut effect = Yufitel::new(0, 250);
        let mut runtime = yufitel_runtime(71);

        run_body(&mut effect, &mut runtime);

        // `Face(10, 0)` from the origin: atan2(10, -0) / 3.141592 * 180.
        let face = ((10.0f64).atan2(0.0) as f32 / 3.141592f32) * 180.0;

        for primitive in &runtime.primitives {
            let state = &primitive.state;
            assert!((state.longitude + face).abs() < 1e-4, "longitude {}", state.longitude);
            assert!((state.render_angle - (180.0 - face)).abs() < 1e-4);
            // `speed = -(CalcDist / duration)` = -(10 / 20).
            assert_eq!(state.speed, -(10.0 / 20.0));
            // `MakeYRotation(GetRadian((int)longitude))`.
            let mut expected = Matrix::identity();
            expected.make_y_rotation(Angles::shared().radian(state.longitude as i32));
            assert_eq!(state.transform.values, expected.values);
        }
    }

    #[test]
    fn yufitel_kind_one_uses_the_twirl_texture_set() {
        let mut effect = Yufitel::new(1, 250);
        let mut runtime = yufitel_runtime(71);

        run_body(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\twirl_soft.bmp"));
        assert_eq!(runtime.primitives[1].state.texture_frames, vec![
            "effect\\twirl_soft.bmp",
            "effect\\thunder_ball_b.bmp",
            "effect\\twirl_soft.bmp",
            "effect\\thunder_ball_d.bmp",
            "effect\\twirl_soft.bmp",
            "effect\\thunder_ball_f.bmp",
        ]);
    }

    #[test]
    fn yufitel_spawns_nothing_after_frame_zero() {
        let mut effect = Yufitel::new(0, 250);
        let mut runtime = yufitel_runtime(71);

        step(&mut effect, &mut runtime);
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 2);
        assert_eq!(runtime.sounds.len(), 1);
    }

    // -- YufitelHit ---------------------------------------------------------

    #[test]
    fn yufitel_hit_spawns_the_flash_and_the_frame_ten_blast() {
        let mut effect = YufitelHit::new(0, 250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 73);

        run_body(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 1);

        runtime.frame = 10;
        run_body(&mut effect, &mut runtime);
        assert_eq!(runtime.primitives.len(), 2);

        let flash = &runtime.primitives[0].state;
        assert_eq!(flash.count, PrimitiveId::Texture3d);
        assert_eq!(flash.duration, 10);
        assert_eq!(flash.pattern, 0x40);
        assert_eq!(flash.render_flag, 1 | 2 | 0x200);
        assert_eq!(flash.delta_pos2[1], -5.0);
        assert_eq!(flash.width_speed, 2.5);
        assert_eq!(flash.height_speed, 2.5);
        assert_eq!(flash.change_point, 10);
        assert_eq!(flash.change_width_speed, 0.0);
        assert_eq!(flash.change_height_speed, 0.0);
        // `-(0.0f / 50.0f)` is negative zero.
        assert!(flash.change_width_accel.is_sign_negative());
        assert_eq!(flash.change_width_accel, 0.0);
        assert!(flash.change_height_accel.is_sign_negative());
        assert_eq!(flash.fade_out_cnt, 1);
        assert_eq!(flash.texture, Some("effect\\thunder_pang.bmp"));

        let blast = &runtime.primitives[1].state;
        assert_eq!(blast.count, PrimitiveId::Texture3d);
        assert_eq!(blast.duration, 300);
        assert_eq!(blast.pattern, 0x40);
        assert_eq!(blast.render_flag, 1 | 0x200 | 2);
        assert_eq!(blast.delta_pos2[1], -5.0);
        assert_eq!(blast.repeat_anim, 1);
        assert_eq!(blast.width_size, 7.5);
        assert_eq!(blast.height_size, 7.5);
        assert_eq!(blast.anim_speed, 1);
        assert_eq!(blast.total_texture, 5);
        assert_eq!(blast.frame_count, 5);
        assert_eq!(blast.texture_frames, vec![
            "effect\\thunder_plazma_blast_a.bmp",
            "effect\\thunder_plazma_blast_b.bmp",
            "effect\\thunder_ball_d.bmp",
            "effect\\thunder_ball_e.bmp",
            "effect\\thunder_ball_f.bmp",
        ]);
        assert_eq!(blast.texture, Some("effect\\thunder_plazma_blast_a.bmp"));
    }

    #[test]
    fn yufitel_hit_flashes_every_twenty_frames() {
        let mut effect = YufitelHit::new(0, 250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 73);

        for _ in 0..250 {
            if !step(&mut effect, &mut runtime) {
                break;
            }
        }

        // Flash on 0, 20, ..., 240 (13) plus the frame-10 blast.
        assert_eq!(runtime.primitives.len(), 14);
        assert_eq!(runtime.primitives.iter().filter(|p| p.state.duration == 300).count(), 1);
    }

    #[test]
    fn yufitel_hit_kind_one_swaps_the_five_blast_frames() {
        let mut effect = YufitelHit::new(1, 250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 73);

        run_body(&mut effect, &mut runtime);
        runtime.frame = 10;
        run_body(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives[0].state.texture, Some("effect\\pokjuk_d.bmp"));
        assert_eq!(runtime.primitives[1].state.texture_frames, vec![
            "effect\\twirl_soft.bmp",
            "effect\\thunder_ball_b.bmp",
            "effect\\twirl_soft.bmp",
            "effect\\thunder_ball_c.bmp",
            "effect\\twirl_soft.bmp",
        ]);
    }

    // -- HitDark ------------------------------------------------------------

    #[test]
    fn hit_dark_spawns_the_ring_and_four_sparks() {
        let mut effect = HitDark::new(50);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 811);
        runtime.parent_angle = 90.0;

        run_body(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 5);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::Cylinder3d);
        assert_eq!(ring.duration, 10);
        assert_eq!(ring.longitude, 90.0, "180 - parent angle");
        assert_eq!(ring.latitude, -90.0);
        assert_eq!(ring.delta_pos2, [0.0, -10.0, 0.0]);
        assert_eq!(ring.speed, 0.7);
        assert_eq!(ring.accel, -(0.7 / 10.0) * DEPTH_AVERAGE_HALF);
        assert_eq!(ring.outer_size, 10.0);
        assert_eq!(ring.inner_size, 5.0);
        assert_eq!(ring.height_size, 3.5);
        assert_eq!(ring.fade_out_cnt, 5);
        assert_eq!(ring.texture, Some("effect\\ring_blue.tga"));

        for spark in &runtime.primitives[1..3] {
            let state = &spark.state;
            assert_eq!(state.count, PrimitiveId::Particle3d);
            assert_eq!((state.red, state.green, state.blue), (20, 20, 20));
            assert_eq!(state.fade_out_cnt, 0);
            assert_eq!(state.num_segment, 3);
            assert_eq!(state.records.len(), 3);
            assert_eq!(state.sprite, Some("이팩트\\particle1.spr"));
            assert_eq!(state.action, Some("이팩트\\particle1.act"));
            assert!((6..=29).contains(&state.duration));
            assert!((0.6..=1.59).contains(&state.size));
            assert!((0.6..=1.49).contains(&state.speed));
            assert!((50.0..=129.0).contains(&state.longitude));
            assert!((-50.0..=49.0).contains(&state.latitude));
        }

        for spark in &runtime.primitives[3..] {
            let state = &spark.state;
            assert_eq!(state.count, PrimitiveId::Particle3dGravity);
            assert_eq!((state.red, state.green, state.blue), (20, 20, 20));
            assert_eq!(state.fade_out_cnt, 0);
            assert_eq!(state.num_segment, 3);
            assert_eq!(state.records.len(), 3);
            assert_eq!(state.sprite, Some("이팩트\\particle1.spr"));
            assert!((230.0..=319.0).contains(&state.longitude));
            assert!((-50.0..=49.0).contains(&state.latitude));
            // gravSpeed spans -1.19 .. -0.30, gravAccel is positive.
            assert!((-1.19..=-0.30).contains(&state.grav_speed));
            assert!(state.grav_accel > 0.0);
        }
    }

    #[test]
    fn hit_dark_replays_the_official_rng_order() {
        let seed = 811;
        let mut effect = HitDark::new(50);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], seed);
        runtime.parent_angle = 90.0;

        run_body(&mut effect, &mut runtime);

        let angles = Angles::shared();
        let mut rng = EffectRng::new(seed);
        let facing = 90;

        // First particle3d spark: 80, 100, 4, 90, 24, 100.
        let longitude = (rng.clamped_random(80) + (facing - 40)) as f32;
        let latitude = (rng.clamped_random(100) - 50) as f32;
        let travel = (rng.clamped_random(4) + 4) as f32;
        let speed = (rng.clamped_random(90) + 60) as f32 / 100.0;
        let duration = rng.clamped_random(24) + 6;
        let size = (rng.clamped_random(100) + 60) as f32 / 100.0;

        let spark = &runtime.primitives[1].state;
        assert_eq!(spark.longitude, longitude);
        assert_eq!(spark.latitude, latitude);
        assert_eq!(spark.duration, duration);
        assert_eq!(spark.speed, speed);
        assert_eq!(spark.size, size);
        assert_eq!(spark.accel, -(speed / duration as f32) * DEPTH_AVERAGE_HALF);

        let mut transform = Matrix::identity();
        transform.make_y_rotation(angles.radian(longitude as i32));
        transform.append_x_rotation(angles.radian(latitude as i32));
        let placed = matrix_mult([0.0, 0.0, travel], &transform.values);
        assert_eq!(spark.delta_pos3, placed);
        assert_eq!(spark.delta_pos2, [placed[0] + 0.0, placed[1] - 10.0, placed[2] + 0.0]);

        // Second particle3d spark consumes the same six draws.
        let _ = rng.clamped_random(80);
        let _ = rng.clamped_random(100);
        let _ = rng.clamped_random(4);
        let _ = rng.clamped_random(90);
        let _ = rng.clamped_random(24);
        let _ = rng.clamped_random(100);

        // First gravity spark: 80, 100, 4, 24, 90, 90, 100.
        let g_longitude = (rng.clamped_random(80) + (facing + 140)) as f32;
        let g_latitude = (rng.clamped_random(100) - 50) as f32;
        let g_travel = (rng.clamped_random(4) + 4) as f32;
        let g_duration = rng.clamped_random(24) + 6;
        let g_speed = (rng.clamped_random(90) + 60) as f32 / 100.0;
        let g_gravity = -(rng.clamped_random(90) + 30) as f32 / 100.0;
        let g_size = (rng.clamped_random(100) + 60) as f32 / 100.0;

        let gravity = &runtime.primitives[3].state;
        assert_eq!(gravity.longitude, g_longitude);
        assert_eq!(gravity.latitude, g_latitude);
        assert_eq!(gravity.duration, g_duration);
        assert_eq!(gravity.speed, g_speed);
        assert_eq!(gravity.grav_speed, g_gravity);
        assert_eq!(gravity.grav_accel, -(g_gravity / g_duration as f32 * 2.0));
        assert_eq!(gravity.size, g_size);

        // The gravity spark keeps only the Y rotation of its heading.
        let mut yaw = Matrix::identity();
        yaw.make_y_rotation(angles.radian(g_longitude as i32));
        assert_eq!(gravity.transform.values, yaw.values);

        let mut g_transform = Matrix::identity();
        g_transform.make_y_rotation(angles.radian(g_longitude as i32));
        g_transform.append_x_rotation(angles.radian(g_latitude as i32));
        let placed = matrix_mult([0.0, 0.0, g_travel], &g_transform.values);
        assert_eq!(gravity.delta_pos3, placed);
        assert_eq!(gravity.delta_pos2, [placed[0] + 0.0, placed[1] - 10.0, placed[2] + 0.0]);
    }

    #[test]
    fn hit_dark_facing_wraps_negative_angles() {
        let mut effect = HitDark::new(50);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 811);
        runtime.parent_angle = 270.0;

        run_body(&mut effect, &mut runtime);

        // 180 - 270 = -90, wrapped to 270.
        assert_eq!(runtime.primitives[0].state.longitude, 270.0);
    }

    // -- HealSP -------------------------------------------------------------

    #[test]
    fn heal_sp_spawns_three_columns_and_overrides_the_duration() {
        let mut effect = HealSp::new(50);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 97);

        run_body(&mut effect, &mut runtime);

        assert_eq!(runtime.lifetime, 100, "state.duration = 100");
        assert_eq!(runtime.sounds, vec!["_heal_effect.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 4, "three cylinders + one orbit");

        let expected = [(2.5f32, 15.0f32, 16.0f32), (3.5, 25.0, 32.0), (4.5, 35.0, 64.0)];

        for (primitive, (outer, height, max_alpha)) in runtime.primitives[..3].iter().zip(expected) {
            let state = &primitive.state;
            assert_eq!(state.count, PrimitiveId::Cylinder3d);
            assert_eq!(state.duration, 100);
            assert_eq!(state.long_speed, 5.0);
            assert_eq!(state.outer_size, outer);
            assert_eq!(state.inner_size, outer);
            assert_eq!(state.height_size, height);
            assert_eq!((state.red, state.green, state.blue), (0x20, 0xB0, 0xE8));
            assert_eq!(state.max_alpha, max_alpha);
            assert_eq!(state.alpha_speed, max_alpha / 30.0);
            assert_eq!(state.fade_out_cnt, 80);
            assert_eq!(state.texture, Some("effect\\alpha_down.tga"));
        }

        let orbit = &runtime.primitives[3].state;
        assert_eq!(orbit.count, PrimitiveId::Orbit);
        assert_eq!(orbit.radius, 3.0);
        assert_eq!(orbit.duration, 30);
        assert_eq!(orbit.grav_speed, -1.2);
        assert_eq!(orbit.grav_accel, -(-1.2 / 30.0) / 1.5);
        assert_eq!(orbit.size, 0.55);
        assert_eq!(orbit.fade_out_cnt, 20);
        assert_eq!(orbit.sprite, Some("이팩트\\particle2.spr"));
        assert_eq!(orbit.action, Some("이팩트\\particle2.act"));
        assert_eq!(orbit.num_segment, 1);

        // The mote hangs off the orbit radius instead of the body centre.
        let mut transform = Matrix::identity();
        transform.make_y_rotation(Angles::shared().radian(orbit.longitude as i32));
        assert_eq!(orbit.delta_pos2, matrix_mult([0.0, 0.0, 3.0], &transform.values));
    }

    #[test]
    fn heal_sp_emits_an_orbit_on_every_even_frame() {
        let mut effect = HealSp::new(50);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], 97);

        for _ in 0..10 {
            step(&mut effect, &mut runtime);
        }

        // Frames 0, 2, 4, 6, 8 -> five orbits plus the three frame-0 columns.
        assert_eq!(runtime.primitives.len(), 8);
        assert_eq!(
            runtime.primitives.iter().filter(|p| p.state.count == PrimitiveId::Orbit).count(),
            5
        );
    }

    #[test]
    fn heal_sp_orbit_discards_one_rand_before_the_heading() {
        let seed = 97;
        let mut effect = HealSp::new(50);
        let mut runtime = EffectRuntime::new(50, [0.0, 0.0], seed);

        run_body(&mut effect, &mut runtime);

        let mut rng = EffectRng::new(seed);
        rng.discard();
        let longitude = rng.clamped_random(360) as f32;

        assert_eq!(runtime.primitives[3].state.longitude, longitude);
    }

    // -- WarpZone -----------------------------------------------------------

    #[test]
    fn warp_zone_spawns_the_disc_pulse_and_orbit_on_frame_zero() {
        let mut effect = WarpZone::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 601);

        run_body(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 3);

        let disc = &runtime.primitives[0].state;
        assert_eq!(disc.count, PrimitiveId::Circle3d);
        assert_eq!(disc.duration, 158);
        assert_eq!(disc.pattern, 1);
        assert_eq!(disc.radius, 15.0);
        assert_eq!(disc.delta_pos2[1], -1.0);
        assert_eq!(disc.latitude, 90.0);
        assert_eq!(disc.max_alpha, 128.0);
        assert_eq!(disc.alpha_speed, 128.0 / 10.0);
        assert_eq!(disc.fade_out_cnt, 148);
        assert_eq!(disc.texture, Some("effect\\alpha_down.tga"));

        let pulse = &runtime.primitives[1].state;
        assert_eq!(pulse.count, PrimitiveId::Circle3d);
        assert_eq!(pulse.duration, 80);
        assert_eq!(pulse.inner_size, 4.0);
        assert_eq!(pulse.latitude, 90.0);
        assert_eq!(pulse.delta_pos2[1], -1.0);
        assert_eq!(pulse.radius, 14.0);
        assert_eq!(pulse.radius_speed, -0.15);
        assert_eq!(pulse.alpha, 5.0);
        assert_eq!(pulse.max_alpha, 200.0);
        assert_eq!(pulse.alpha_speed, 10.0);
        assert_eq!(pulse.ch_point, vec![20.0]);
        assert_eq!(pulse.ch_val1, vec![0.0]);
        assert_eq!(pulse.fade_out_cnt, 64);
        assert_eq!(pulse.texture, Some("effect\\ring_blue.tga"));

        let mote = &runtime.primitives[2].state;
        assert_eq!(mote.count, PrimitiveId::Orbit);
        assert_eq!(mote.duration, 130);
        assert_eq!(mote.radius_speed, 0.0005);
        assert_eq!(mote.long_speed, 2.0);
        assert_eq!(mote.grav_speed, -0.2);
        assert_eq!(mote.grav_accel, -(-0.2 / 130.0) / 1.5);
        // `ClampedRandom(200) / 100` is 0 or 1 -> radius 5.0 or 5.75.
        assert!(mote.radius == 5.0 || mote.radius == 5.75, "radius {}", mote.radius);
        // Integer division makes the size 0.7 for every draw.
        assert_eq!(mote.size, 0.7);
        assert_eq!(mote.fade_out_cnt, 87);
        assert_eq!(mote.sprite, Some("이팩트\\particle1.spr"));
        assert_eq!(mote.action, Some("이팩트\\particle1.act"));

        let mut transform = Matrix::identity();
        transform.make_y_rotation(Angles::shared().radian(mote.longitude as i32));
        assert_eq!(mote.delta_pos2, matrix_mult([0.0, 0.0, mote.radius], &transform.values));
    }

    #[test]
    fn warp_zone_orbit_replays_the_official_rng_order() {
        let seed = 601;
        let mut effect = WarpZone::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], seed);

        run_body(&mut effect, &mut runtime);

        let mut rng = EffectRng::new(seed);
        let radius = (rng.clamped_random(200) / 100) as f32 * 0.75 + 5.0;
        let longitude = rng.clamped_random(360) as f32;
        rng.discard();
        let size = (rng.clamped_random(10) / 10) as f32 + 0.7;

        let mote = &runtime.primitives[2].state;
        assert_eq!(mote.radius, radius);
        assert_eq!(mote.longitude, longitude);
        assert_eq!(mote.size, size);
    }

    #[test]
    fn warp_zone_pulses_on_its_frame_cadence() {
        let mut effect = WarpZone::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 601);

        for _ in 0..29 {
            step(&mut effect, &mut runtime);
        }

        // Frames 0..28: pulses on 0 and 28 (28 % 10 != 0), orbits on 0, 10, 20.
        assert_eq!(
            runtime
                .primitives
                .iter()
                .filter(|p| p.state.count == PrimitiveId::Circle3d && p.state.inner_size == 4.0)
                .count(),
            2
        );
        assert_eq!(
            runtime.primitives.iter().filter(|p| p.state.count == PrimitiveId::Orbit).count(),
            3,
            "orbits on 0, 10, 20"
        );
    }

    #[test]
    fn warp_zone_restarts_its_counter_on_frame_145() {
        let mut effect = WarpZone::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 601);

        while runtime.frame < 145 {
            step(&mut effect, &mut runtime);
        }

        // The next step reads stateCnt 145, resets it to 0 and spawns the
        // second disc.
        step(&mut effect, &mut runtime);

        assert_eq!(runtime.frame, 1, "stateCnt reset to 0 then incremented");
        assert_eq!(
            runtime
                .primitives
                .iter()
                .filter(|p| p.state.count == PrimitiveId::Circle3d && p.state.duration == 158)
                .count(),
            2
        );
    }
}
