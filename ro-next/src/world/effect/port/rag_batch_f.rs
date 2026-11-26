//! Effects ported by the "f" parallel batch.
//!
//! This file is owned exclusively by batch "f": bodies and their tests live
//! here so parallel work never edits a shared module. Every body is a direct
//! transliteration of one decompiled `CRagEffect` function (or one inline
//! `OnProcess` case); the frame gates, literal constants, RNG consumption
//! order and the official quirks are kept, including the dead branch of
//! `TurnUndead`'s `loopCount`.
//!
//! Two official side effects have no channel in the hermetic runtime and are
//! documented at their call site: `Sightrasher`'s
//! `AM_DELETE_EFFECT / EF_SIGHT` parent message (it removes the caster's Sight
//! overlay) and the per-frame `g_renderer->Project` writes of `tlvertX/Y` (the
//! runtime already carries the projected anchor in
//! [`EffectRuntime::screen`](super::runtime::EffectRuntime::screen)).

#![allow(dead_code)]

use super::angles::Angles;
use super::math::matrix_mult;
use super::primitive::PrimitiveId;
use super::runtime::{EffectRuntime, PortedEffect};

// ---------------------------------------------------------------------------
// EF_SIGHTRASHER
// ---------------------------------------------------------------------------

/// `CRagEffect::Sightrasher` - `RagEffect.cpp:11305-11397`.
///
/// Dispatch: `case EF_SIGHTRASHER: this->Sightrasher();` -
/// `RagEffect.cpp:23855-23857`.
///
/// On frames 0, 5, 10 and 15 (every fifth frame while `stateCnt < 20`) it
/// throws two eight-particle fans: eight `PP_3DPARTICLE` `Sight.spr` motes
/// with the `0x400` free-flight bit and eight `Shadow.spr` motes. On frame 10
/// it also lays the `ring_yellow.tga` ground ring. The radius and the vertical
/// offset of each fan step outward as `stateCnt` grows, but every mote keeps
/// the `85 - stateCnt` / `65 - stateCnt` lifetime pair.
///
/// Quirks kept: `GetRadian(angle)` is evaluated and discarded; the y of the
/// first fan is `stateCnt / 3 - 20` (integer division) while the shadow fan
/// uses `0`; the shadow fan writes only `changeAccel3d.x/.z` in the original,
/// so its `y` stays at the constructor zero. The port's constructor still
/// starts the two `change*3d` triples at `0.0001`, so this body writes the
/// zero components explicitly (`EffectPrim.cpp:98-103`).
///
/// The frame-0 `state.parent->SendMsg(nullptr, AM_DELETE_EFFECT, EF_SIGHT,
/// ...)` removes the caster's Sight overlay. The port has no actor-message
/// channel (see the module note), so the request is not simulated.
#[derive(Debug, Default)]
pub struct Sightrasher {}

/// One eight-mote fan: the two sprite variants differ only in these fields.
fn sightrasher_fan(runtime: &mut EffectRuntime, frame: i32, shadow: bool) {
    let angles = Angles::shared();

    for index in 0..8 {
        // `static_cast<int>(static_cast<float>(i) * -45.0f)`.
        let angle = (index as f32 * -45.0) as i32;
        let sine = angles.sine(angle);
        let cosine = angles.cosine(angle);

        // `ClampedRandom(2) + 3` - the only RNG draw, and only for the Sight
        // fan; the shadow fan hard-codes 3.
        let anim_speed = if shadow { 3 } else { runtime.rng.clamped_random(2) + 3 };

        let duration = 85 - frame;
        let reach = (frame / 5) as f32 + 5.0;
        let delta_y = if shadow { 0.0 } else { (frame / 3 - 20) as f32 };
        let size = if shadow {
            1.0 - frame as f32 / 30.0
        } else {
            2.5 - frame as f32 / 10.0
        };

        let instance = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(instance);

        state.duration = duration;
        state.pattern |= 0x400;
        state.delta_pos2 = [reach * sine, delta_y, -reach * cosine];
        state.speed3d = [sine, 0.0, cosine * -1.0];
        state.anim_speed = anim_speed;
        state.size = size;
        state.alpha = ((75 - frame) * 2) as f32;
        state.change_speed3d = [sine * 0.5, 0.0, cosine * -0.5];
        state.change_point = 35 - frame;
        state.fade_out_cnt = 65 - frame;

        // `changeSpeed3d` was written above; the shadow fan's y and the
        // unset components must be the C++ constructor's zero (the port
        // constructor uses 0.0001 for both triples).
        if shadow {
            // QUIRK: the shadow fan writes only `changeAccel3d.x/.z`; `y`
            // keeps the constructor zero and `deltaPosAccel` is untouched.
            state.change_accel3d = [-0.004, 0.0, 0.004];
            state.sprite = Some("Shadow.spr");
            state.action = Some("Shadow.act");
        } else {
            state.delta_pos_accel = [0.0, 0.0, 0.0];
            state.change_accel3d = [-0.004, 0.0, 0.004];
            state.sprite = Some("이팩트\\Sight.spr");
            state.action = Some("이팩트\\Sight.act");
        }

        state.init_segment();
    }
}

impl PortedEffect for Sightrasher {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:71` - `result[EF_SIGHTRASHER] = 250`.
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 0 {
            // `AM_DELETE_EFFECT, EF_SIGHT` has no runtime channel (module note).
            runtime.play_sound("effect\\wizard_sightrasher.wav");
        }

        if frame % 5 == 0 && frame < 20 {
            sightrasher_fan(runtime, frame, false);
            sightrasher_fan(runtime, frame, true);
        }

        if frame == 10 {
            let ring = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(ring);

            state.duration = 30;
            state.latitude = 90.0;
            state.radius_speed = 2.5;
            state.radius_accel = -(state.radius_speed / state.duration as f32) / 1.5;
            state.inner_size = -10.0;
            state.alpha = 150.0;
            state.fade_out_cnt = state.duration - 10;
            state.init_segment();
            state.texture = Some("effect\\ring_yellow.tga");
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_TURNUNDEAD
// ---------------------------------------------------------------------------

/// `CRagEffect::TurnUndead` - `RagEffect.cpp:9397-9487`.
///
/// Dispatch: `case EF_TURNUNDEAD: this->TurnUndead();` -
/// `RagEffect.cpp:23901-23903`.
///
/// Frame 0 lays the 370 px `PP_2DCIRCLE` (`alpha_down.tga`, pattern bit 1)
/// plus **five** `PP_2DFLASH` shards that rise from `heightSize ~350`; frame 37
/// plays `EF_Bash.wav`; frame 50 adds the `PP_3DTEXTURE` `ring_liner.tga`
/// plane. The decompiled `int loopCount = 20; if (stateCnt == 0) loopCount =
/// 5;` is a dead branch once inside the `stateCnt == 0` block - the port keeps
/// the resulting five iterations verbatim.
///
/// Quirks kept: the flash fills `speed3d.x` with the parent x and `speed3d.y`
/// with the parent **z**; `chCnt` is `(uint32_t)-1`; the 5-flash loop is
/// followed by a per-frame `render.position = parent->render.position` (the
/// runtime anchor is already that position, so it is a no-op here).
#[derive(Debug, Default)]
pub struct TurnUndead {}

impl PortedEffect for TurnUndead {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:91` - `result[EF_TURNUNDEAD] = 250`.
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 37 {
            runtime.play_sound("effect\\EF_Bash.wav");
        }

        if frame == 0 {
            let ring = runtime.launch(PrimitiveId::Circle2d, [0.0, 0.0, 0.0]);
            {
                let state = runtime.primitive_mut(ring);

                state.radius = 370.0;
                state.radius_speed = -(state.radius / 20.0);
                state.duration = 20;
                state.pattern |= 1;
                state.delta_pos2[1] = -40.0;
                state.alpha = 0.0;
                state.max_alpha = 130.0;
                state.alpha_speed = state.max_alpha / 6.0;
                state.fade_out_cnt = state.duration - 10;
                state.texture = Some("effect\\alpha_down.tga");
            }

            // `loopCount = 20` is overwritten by `5` by the always-true
            // `if (stateCnt == 0)` inside the `stateCnt == 0` block.
            let parent_x = runtime.position[0];
            let parent_z = runtime.position[2];

            for _ in 0..5 {
                // RNG order: longitude, longSpeed, arcAngle, heightSize,
                // heightSpeed.
                let longitude = runtime.rng.clamped_random(360) as f32;
                let long_speed = (runtime.rng.clamped_random(60) + 10) as f32 / 10.0;
                let arc_angle = (runtime.rng.clamped_random(25) + 5) as f32 / 10.0;
                let height_size = (runtime.rng.clamped_random(30) + 350) as f32;
                let height_speed = -((runtime.rng.clamped_random(30) + 150) as f32) / 10.0;

                let flash = runtime.launch(PrimitiveId::Flash2d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(flash);

                state.duration = 20;
                state.ch_cnt = u32::MAX;
                state.speed3d[0] = parent_x;
                // QUIRK: the original copies the parent's **z** into `y`.
                state.speed3d[1] = parent_z;
                state.delta_pos2[1] = -40.0;
                state.longitude = longitude;
                state.long_speed = long_speed;
                state.long_accel = -(state.long_speed / state.duration as f32) / 1.5;
                state.arc_angle = arc_angle;
                state.height_size = height_size;
                state.height_speed = height_speed;
                state.alpha = 0.0;
                state.max_alpha = 150.0;
                state.alpha_speed = state.max_alpha / 10.0;
                state.fade_out_cnt = state.duration - state.duration / 3;
                state.texture = Some("EFFECT\\alpha_center.tga");
            }
        }

        // `CopyVector(this->render.position, parent->render.position)` - the
        // runtime's `position` is already the parent anchor.

        if frame == 50 {
            let plane = runtime.launch(PrimitiveId::Texture3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(plane);

            state.duration = 30;
            state.roll = 90.0;
            state.width_speed = 1.5;
            state.height_speed = 1.5;
            state.delta_pos2[1] = -7.0;
            state.width_accel = state.width_speed / 10.0;
            state.height_accel = state.height_speed / 10.0;
            state.alpha = 150.0;
            state.fade_out_cnt = state.duration - 10;
            state.change_point = 15;
            state.change_width_speed = 0.5;
            state.change_width_accel = -0.5 / 15.0;
            state.change_height_speed = 0.5;
            state.change_height_accel = -0.5 / 15.0;
            state.init_segment();
            state.texture = Some("effect\\ring_liner.tga");
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_HASTEUP
// ---------------------------------------------------------------------------

/// `CRagEffect::HasteUp` - `RagEffect.cpp:12839-12896`.
///
/// Dispatch: `case EF_HASTEUP: this->HasteUp();` - `RagEffect.cpp:23913-23915`.
///
/// Frame 0 writes `state.duration = 300` (the lifetime table says 250, so the
/// effect actually runs 300 frames) and plays the first Adrenaline Rush wave;
/// frame 30 plays the second. The same frame launches twenty
/// `PP_2DFLASH` `alpha_center.tga` shards (80-frame life, upward drift with a
/// frame-40 speed change) and four `PP_3DPARTICLEORBIT` `particle1.spr` motes
/// at radius 7 with a downward gravity of `-0.1` / `-0.002`.
///
/// Quirks kept: the orbit's `longSpeed` is
/// `stateCnt * longAccel + 0.3` - on frame 0 that is exactly `0.3`, but the
/// expression is kept; the flash height speed divides by **13**, not 10.
#[derive(Debug, Default)]
pub struct HasteUp {}

impl PortedEffect for HasteUp {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:107` - `result[EF_HASTEUP] = 250`. The body raises
        // `state.duration` to 300 on frame 0 (`RagEffect.cpp:12841`).
        250
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 0 {
            runtime.lifetime = 300;
            runtime.play_sound("effect\\black_adrenalinerush_a.wav");
        }

        if frame == 30 {
            runtime.play_sound("effect\\black_adrenalinerush_b.wav");
        }

        if frame == 0 {
            let angles = Angles::shared();

            for _ in 0..20 {
                // RNG order: longitude, longSpeed, arcAngle, heightSpeed,
                // changeSpeed.
                let longitude = runtime.rng.clamped_random(360) as f32;
                let long_speed = (runtime.rng.clamped_random(60) + 10) as f32 / 10.0;
                let arc_angle = (runtime.rng.clamped_random(25) + 5) as f32 / 10.0;
                let height_speed = (runtime.rng.clamped_random(20) + 50) as f32 / 13.0;
                let change_speed = (runtime.rng.clamped_random(10) + 15) as f32 / 10.0;

                let flash = runtime.launch(PrimitiveId::Flash2d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(flash);

                state.duration = 80;
                state.delta_pos2[1] = -40.0;
                state.longitude = longitude;
                state.long_speed = long_speed;
                state.long_accel = -(state.long_speed / state.duration as f32) / 1.5;
                state.arc_angle = arc_angle;
                state.height_size = 0.0;
                state.height_speed = height_speed;
                state.change_point = 40;
                state.change_speed = change_speed;
                state.change_accel = 0.0;
                state.change_height_speed = 0.0;
                state.texture = Some("EFFECT\\alpha_center.tga");
            }

            for index in 0..4 {
                let orbit = runtime.launch(PrimitiveId::Orbit, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(orbit);

                state.duration = 100;
                state.radius = 7.0;
                state.grav_speed = -0.1;
                state.grav_accel = -0.002;
                state.longitude = (index * 90) as f32;
                state.long_accel = 0.2;
                // QUIRK: reads the current (frame 0) `stateCnt`.
                state.long_speed = frame as f32 * state.long_accel + 0.3;
                state.size = 1.2;
                state.transform.make_y_rotation(angles.radian(state.longitude as i32));
                state.fade_out_cnt = state.duration - state.duration / 10;
                state.sprite = Some("이팩트\\particle1.spr");
                state.action = Some("이팩트\\particle1.act");
                state.init_segment();
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// EF_DANCE2 (RisingSparkles)
// ---------------------------------------------------------------------------

/// `CRagEffect::RisingSparkles(colorMode)` - `RagEffect.cpp:7078-7122`, driven
/// by the `EF_DANCE2` dispatch block - `RagEffect.cpp:20108-20121`.
///
/// Every even frame launches one `PP_3DCROSSTEXTURE` `ac_center2.tga` spark:
/// a random Y rotation scatters it `ClampedRandom(7) + 3` outward and 5 down,
/// then the transform is replaced by an X rotation of `-90`. `EF_DANCE2` passes
/// `colorMode == 2` (red = random, green = red, blue = 255) and tints the
/// caster purple on even frames 4..90, white on the odd ones, requesting
/// `render.flags` bit 1 while tinted.
///
/// The dispatch writes are the official per-frame actor mutations; the port
/// records them through [`EffectRuntime::set_parent_color`] and
/// [`EffectRuntime::add_parent_flags`], exactly like the other tint bodies.
#[derive(Debug)]
pub struct RisingSparkles {
    color_mode: i32,
}

impl RisingSparkles {
    pub fn new(color_mode: i32) -> Self {
        Self { color_mode }
    }
}

impl PortedEffect for RisingSparkles {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:857` - `result[EF_DANCE2] = 100`.
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        // `case EF_DANCE2:` - `stateCnt >= 4 && stateCnt <= 90`.
        if (4..=90).contains(&frame) {
            if frame % 2 == 0 {
                if runtime.parent_flags & 1 == 0 {
                    runtime.add_parent_flags(1);
                }

                runtime.set_parent_color(-1, 182, 89, 255);
            } else {
                runtime.set_parent_color(-1, 255, 255, 255);
            }
        }

        // `this->render.position = parent.position` - already the runtime
        // anchor.
        if frame & 1 != 0 {
            return true;
        }

        let angles = Angles::shared();

        // RNG order: longitude, distance, width, colour channel.
        let longitude = runtime.rng.clamped_random(360);
        let distance = (runtime.rng.clamped_random(7) + 3) as f32;
        let width = (runtime.rng.clamped_random(60) + 30) as f32 / 10.0;
        let channel = runtime.rng.clamped_random(256);

        let spark = runtime.launch(PrimitiveId::CrossTexture3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(spark);

        state.duration = 50;
        state.transform.make_y_rotation(angles.radian(longitude));
        state.delta_pos2 = matrix_mult([0.0, 0.0, distance], &state.transform.values);
        state.delta_pos2[1] -= 5.0;
        state.transform.make_x_rotation(angles.radian(-90));
        state.accel = -0.015;
        state.width_size = width;
        state.height_size = 0.18;
        state.alpha = 0.0;
        state.max_alpha = 200.0;

        if self.color_mode == 2 {
            state.red = channel;
            state.green = state.red;
            state.blue = 255;
        } else {
            state.red = 255;
            state.green = channel;
            state.blue = state.green;
        }

        state.alpha_speed = state.max_alpha / 20.0;
        state.fade_out_cnt = state.duration - 20;
        state.texture = Some("effect\\ac_center2.tga");
        state.slots[0].life = 0;

        true
    }
}

// ---------------------------------------------------------------------------
// EF_THUNDERSTORM2
// ---------------------------------------------------------------------------

/// `CRagEffect` dispatch `case EF_THUNDERSTORM2` - `RagEffect.cpp:21134-21147`
/// (inline body, no helper).
///
/// Frame 0 plays the positional `EF_ThunderStorm.wav` (`PlayEffectWave`, i.e.
/// height 0) and launches one `PP_3DPARTICLE` `썬더스톰.spr` sprite that lives
/// for the whole effect (`state.duration`), rendered without depth writes
/// (`RSF_NO_DEPTH`) and with pattern bit `0x200`.
#[derive(Debug, Default)]
pub struct Thunderstorm2 {}

impl PortedEffect for Thunderstorm2 {
    fn lifetime(&self) -> i32 {
        // `EffectTypes.cpp:631` - `result[EF_THUNDERSTORM2] = 100`.
        100
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        // `PlayEffectWave` - `RagEffect.cpp:152-158` (height 0).
        runtime.play_sound_at_height("effect\\EF_ThunderStorm.wav", 0.0);

        let lifetime = runtime.lifetime;
        let bolt = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(bolt);

        state.duration = lifetime;
        state.render_flag |= 0x200; // RSF_NO_DEPTH
        state.pattern |= 0x200;
        state.delta_pos2[1] = -4.5;
        state.anim_speed = 2;
        state.size = 2.5;
        state.sprite = Some("이팩트\\썬더스톰.spr");
        state.action = Some("이팩트\\썬더스톰.act");
        state.init_segment();

        true
    }
}

#[cfg(test)]
mod tests {
    use super::super::runtime::{EffectRng, EffectRuntime, run_to_completion, step};
    use super::*;

    fn runtime_with(seed: u64, lifetime: i32) -> EffectRuntime {
        EffectRuntime::new(lifetime, [0.0, 0.0], seed)
    }

    // -- EF_SIGHTRASHER ----------------------------------------------------

    #[test]
    fn sightrasher_frame_zero_spawns_two_eight_mote_fans() {
        let mut effect = Sightrasher::default();
        let mut runtime = runtime_with(62, 250);

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.sounds, vec!["effect\\wizard_sightrasher.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 16);

        // The Sight fan consumes one `ClampedRandom(2)` per mote; the shadow
        // fan consumes none.
        let mut rng = EffectRng::new(62);
        let expected_anim: Vec<i32> = (0..8).map(|_| rng.clamped_random(2) + 3).collect();

        for index in 0..8 {
            let state = &runtime.primitives[index].state;
            assert_eq!(state.count, PrimitiveId::Particle3d);
            assert_eq!(state.duration, 85);
            assert_eq!(state.pattern, 0x400);
            assert_eq!(state.anim_speed, expected_anim[index]);
            assert_eq!(state.size, 2.5);
            assert_eq!(state.alpha, 150.0);
            assert_eq!(state.change_point, 35);
            assert_eq!(state.fade_out_cnt, 65);
            assert_eq!(state.sprite, Some("이팩트\\Sight.spr"));
            assert_eq!(state.action, Some("이팩트\\Sight.act"));
            assert_eq!(state.delta_pos_accel, [0.0, 0.0, 0.0]);
            assert_eq!(state.change_accel3d, [-0.004, 0.0, 0.004]);
            assert_eq!(state.records.len(), 1);
        }

        // Index 0 is the 0-degree mote: sin 0 / cos 1.
        let first = &runtime.primitives[0].state;
        assert_eq!(first.delta_pos2, [0.0, -20.0, -5.0]);
        assert_eq!(first.speed3d, [0.0, 0.0, -1.0]);
        assert_eq!(first.change_speed3d, [0.0, 0.0, -0.5]);

        // The first shadow mote: same geometry, y forced to 0, size 1.
        let shadow = &runtime.primitives[8].state;
        assert_eq!(shadow.duration, 85);
        assert_eq!(shadow.delta_pos2, [0.0, 0.0, -5.0]);
        assert_eq!(shadow.speed3d, [0.0, 0.0, -1.0]);
        assert_eq!(shadow.anim_speed, 3);
        assert_eq!(shadow.size, 1.0);
        assert_eq!(shadow.alpha, 150.0);
        assert_eq!(shadow.change_point, 35);
        assert_eq!(shadow.fade_out_cnt, 65);
        // QUIRK: the shadow path leaves `deltaPosAccel` at the constructor
        // zero and writes only `changeAccel3d.x/.z` (its `y` stays zero).
        assert_eq!(shadow.delta_pos_accel, [0.0, 0.0, 0.0]);
        assert_eq!(shadow.change_accel3d, [-0.004, 0.0, 0.004]);
        assert_eq!(shadow.change_speed3d, [0.0, 0.0, -0.5]);
        assert_eq!(shadow.sprite, Some("Shadow.spr"));
        assert_eq!(shadow.action, Some("Shadow.act"));
    }

    #[test]
    fn sightrasher_frame_five_uses_the_outer_radius_step() {
        let mut effect = Sightrasher::default();
        let mut runtime = runtime_with(62, 250);

        effect.on_process(&mut runtime);
        runtime.frame = 5;
        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 32);

        let state = &runtime.primitives[16].state;
        assert_eq!(state.duration, 80);
        // reach = 5/5 + 5 = 6, y = 5/3 - 20 = -19.
        assert_eq!(state.delta_pos2, [0.0, -19.0, -6.0]);
        assert_eq!(state.size, 2.0);
        assert_eq!(state.alpha, 140.0);
        assert_eq!(state.change_point, 30);
        assert_eq!(state.fade_out_cnt, 60);
    }

    #[test]
    fn sightrasher_frame_ten_adds_the_yellow_ground_ring() {
        let mut effect = Sightrasher::default();
        let mut runtime = runtime_with(62, 250);

        for frame in [0, 5, 10] {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 49);

        let ring = &runtime.primitives[48].state;
        assert_eq!(ring.count, PrimitiveId::Circle3d);
        assert_eq!(ring.duration, 30);
        assert_eq!(ring.latitude, 90.0);
        assert_eq!(ring.radius_speed, 2.5);
        assert_eq!(ring.radius_accel, -(2.5 / 30.0) / 1.5);
        assert_eq!(ring.inner_size, -10.0);
        assert_eq!(ring.alpha, 150.0);
        assert_eq!(ring.fade_out_cnt, 20);
        assert_eq!(ring.texture, Some("effect\\ring_yellow.tga"));
        assert_eq!(ring.records.len(), 1);
    }

    #[test]
    fn sightrasher_stops_emitting_after_fifteen_and_runs_250_frames() {
        let mut effect = Sightrasher::default();
        let mut runtime = runtime_with(62, 250);

        for frame in 0..20 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        assert_eq!(runtime.primitives.len(), 65, "frames 0/5/10/15 only");

        let mut effect = Sightrasher::default();
        let mut runtime = runtime_with(62, 250);
        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 250);
    }

    // -- EF_TURNUNDEAD -----------------------------------------------------

    #[test]
    fn turnundead_frame_zero_spawns_the_ring_and_five_flashes() {
        let mut effect = TurnUndead::default();
        let mut runtime = runtime_with(82, 250);
        runtime.position = [10.0, 20.0, 30.0];

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.primitives.len(), 6);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::Circle2d);
        assert_eq!(ring.radius, 370.0);
        assert_eq!(ring.radius_speed, -18.5);
        assert_eq!(ring.duration, 20);
        assert_eq!(ring.pattern, 1);
        assert_eq!(ring.delta_pos2[1], -40.0);
        assert_eq!(ring.alpha, 0.0);
        assert_eq!(ring.max_alpha, 130.0);
        assert_eq!(ring.alpha_speed, 130.0 / 6.0);
        assert_eq!(ring.fade_out_cnt, 10);
        assert_eq!(ring.texture, Some("effect\\alpha_down.tga"));

        // RNG order for each flash: longitude, longSpeed, arcAngle,
        // heightSize, heightSpeed.
        let mut rng = EffectRng::new(82);
        let longitude = rng.clamped_random(360) as f32;
        let long_speed = (rng.clamped_random(60) + 10) as f32 / 10.0;
        let arc_angle = (rng.clamped_random(25) + 5) as f32 / 10.0;
        let height_size = (rng.clamped_random(30) + 350) as f32;
        let height_speed = -((rng.clamped_random(30) + 150) as f32) / 10.0;

        let flash = &runtime.primitives[1].state;
        assert_eq!(flash.count, PrimitiveId::Flash2d);
        assert_eq!(flash.duration, 20);
        assert_eq!(flash.ch_cnt, u32::MAX);
        // QUIRK: x gets the parent x, y gets the parent z.
        assert_eq!(flash.speed3d, [10.0, 30.0, 0.0]);
        assert_eq!(flash.delta_pos2[1], -40.0);
        assert_eq!(flash.longitude, longitude);
        assert_eq!(flash.long_speed, long_speed);
        assert_eq!(flash.long_accel, -(long_speed / 20.0) / 1.5);
        assert_eq!(flash.arc_angle, arc_angle);
        assert_eq!(flash.height_size, height_size);
        assert_eq!(flash.height_speed, height_speed);
        assert_eq!(flash.alpha, 0.0);
        assert_eq!(flash.max_alpha, 150.0);
        assert_eq!(flash.alpha_speed, 15.0);
        assert_eq!(flash.fade_out_cnt, 14);
        assert_eq!(flash.texture, Some("EFFECT\\alpha_center.tga"));
    }

    #[test]
    fn turnundead_plays_the_bash_wave_on_frame_thirty_seven() {
        let mut effect = TurnUndead::default();
        let mut runtime = runtime_with(82, 250);

        for frame in 0..38 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        assert_eq!(runtime.sounds, vec!["effect\\EF_Bash.wav".to_owned()]);
    }

    #[test]
    fn turnundead_adds_the_ring_liner_plane_on_frame_fifty() {
        let mut effect = TurnUndead::default();
        let mut runtime = runtime_with(82, 250);
        runtime.frame = 50;

        effect.on_process(&mut runtime);

        assert_eq!(runtime.primitives.len(), 1);
        let plane = &runtime.primitives[0].state;
        assert_eq!(plane.count, PrimitiveId::Texture3d);
        assert_eq!(plane.duration, 30);
        assert_eq!(plane.roll, 90.0);
        assert_eq!(plane.width_speed, 1.5);
        assert_eq!(plane.height_speed, 1.5);
        assert_eq!(plane.delta_pos2[1], -7.0);
        assert_eq!(plane.width_accel, 0.15);
        assert_eq!(plane.height_accel, 0.15);
        assert_eq!(plane.alpha, 150.0);
        assert_eq!(plane.fade_out_cnt, 20);
        assert_eq!(plane.change_point, 15);
        assert_eq!(plane.change_width_speed, 0.5);
        assert_eq!(plane.change_width_accel, -0.5 / 15.0);
        assert_eq!(plane.change_height_speed, 0.5);
        assert_eq!(plane.change_height_accel, -0.5 / 15.0);
        assert_eq!(plane.texture, Some("effect\\ring_liner.tga"));
        assert_eq!(plane.records.len(), 1);
    }

    // -- EF_HASTEUP --------------------------------------------------------

    #[test]
    fn hasteup_frame_zero_extends_the_lifetime_and_spawns_24_primitives() {
        let mut effect = HasteUp::default();
        let mut runtime = runtime_with(98, 250);

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.lifetime, 300);
        assert_eq!(runtime.sounds, vec!["effect\\black_adrenalinerush_a.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 24);

        // RNG order for each flash: longitude, longSpeed, arcAngle,
        // heightSpeed, changeSpeed.
        let mut rng = EffectRng::new(98);
        let longitude = rng.clamped_random(360) as f32;
        let long_speed = (rng.clamped_random(60) + 10) as f32 / 10.0;
        let arc_angle = (rng.clamped_random(25) + 5) as f32 / 10.0;
        let height_speed = (rng.clamped_random(20) + 50) as f32 / 13.0;
        let change_speed = (rng.clamped_random(10) + 15) as f32 / 10.0;

        let flash = &runtime.primitives[0].state;
        assert_eq!(flash.count, PrimitiveId::Flash2d);
        assert_eq!(flash.duration, 80);
        assert_eq!(flash.delta_pos2[1], -40.0);
        assert_eq!(flash.longitude, longitude);
        assert_eq!(flash.long_speed, long_speed);
        assert_eq!(flash.long_accel, -(long_speed / 80.0) / 1.5);
        assert_eq!(flash.arc_angle, arc_angle);
        assert_eq!(flash.height_size, 0.0);
        assert_eq!(flash.height_speed, height_speed);
        assert_eq!(flash.change_point, 40);
        assert_eq!(flash.change_speed, change_speed);
        assert_eq!(flash.change_accel, 0.0);
        assert_eq!(flash.change_height_speed, 0.0);
        assert_eq!(flash.texture, Some("EFFECT\\alpha_center.tga"));
        // The body writes no alpha fields; the constructor's 254 survives.
        assert_eq!(flash.alpha, 254.0);
        assert_eq!(flash.max_alpha, 254.0);

        let orbit = &runtime.primitives[20].state;
        assert_eq!(orbit.count, PrimitiveId::Orbit);
        assert_eq!(orbit.duration, 100);
        assert_eq!(orbit.radius, 7.0);
        assert_eq!(orbit.grav_speed, -0.1);
        assert_eq!(orbit.grav_accel, -0.002);
        assert_eq!(orbit.longitude, 0.0);
        assert_eq!(orbit.long_accel, 0.2);
        // QUIRK: stateCnt * longAccel + 0.3, evaluated on frame 0.
        assert_eq!(orbit.long_speed, 0.3);
        assert_eq!(orbit.size, 1.2);
        assert_eq!(orbit.fade_out_cnt, 90);
        assert_eq!(orbit.sprite, Some("이팩트\\particle1.spr"));
        assert_eq!(orbit.action, Some("이팩트\\particle1.act"));
        assert_eq!(orbit.records.len(), 1);

        assert_eq!(runtime.primitives[21].state.longitude, 90.0);
        assert_eq!(runtime.primitives[23].state.longitude, 270.0);
    }

    #[test]
    fn hasteup_plays_the_second_wave_on_frame_thirty() {
        let mut effect = HasteUp::default();
        let mut runtime = runtime_with(98, 250);

        for frame in 0..31 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        assert_eq!(runtime.sounds, vec![
            "effect\\black_adrenalinerush_a.wav".to_owned(),
            "effect\\black_adrenalinerush_b.wav".to_owned(),
        ]);
    }

    #[test]
    fn hasteup_runs_the_extended_300_frame_duration() {
        let mut effect = HasteUp::default();
        let mut runtime = runtime_with(98, 250);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        // The lifetime table says 250; the frame-0 write extends it to 300.
        assert_eq!(snapshots.len(), 300);
        assert_eq!(runtime.lifetime, 300);
    }

    // -- EF_DANCE2 ---------------------------------------------------------

    #[test]
    fn dance2_frame_zero_spawns_one_purple_variant_spark() {
        let mut effect = RisingSparkles::new(2);
        let mut runtime = runtime_with(848, 100);

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.primitives.len(), 1);
        // The tint block starts on frame 4, not frame 0.
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);

        let mut rng = EffectRng::new(848);
        let longitude = rng.clamped_random(360);
        let distance = (rng.clamped_random(7) + 3) as f32;
        let width = (rng.clamped_random(60) + 30) as f32 / 10.0;
        let channel = rng.clamped_random(256);

        let spark = &runtime.primitives[0].state;
        assert_eq!(spark.count, PrimitiveId::CrossTexture3d);
        assert_eq!(spark.duration, 50);
        assert_eq!(spark.accel, -0.015);
        assert_eq!(spark.width_size, width);
        assert_eq!(spark.height_size, 0.18);
        assert_eq!(spark.alpha, 0.0);
        assert_eq!(spark.max_alpha, 200.0);
        assert_eq!(spark.alpha_speed, 10.0);
        assert_eq!(spark.fade_out_cnt, 30);
        assert_eq!(spark.texture, Some("effect\\ac_center2.tga"));
        assert_eq!(spark.slots[0].life, 0);
        // colorMode == 2: red = rand() % 256, green = red, blue = 255.
        assert_eq!(spark.red, channel);
        assert_eq!(spark.green, channel);
        assert_eq!(spark.blue, 255);

        // The scatter is the Y-rotated distance, dropped by 5.
        assert_eq!(spark.delta_pos2[1], -5.0);
        let sine = Angles::shared().sine(longitude);
        let cosine = Angles::shared().cosine(longitude);
        assert!((spark.delta_pos2[0] - distance * sine).abs() < 1e-4);
        assert!((spark.delta_pos2[2] - distance * cosine).abs() < 1e-4);
        // The transform is then replaced by MakeXRotation(GetRadian(-90)).
        // QUIRK: `GetRadian(-90)` reads the 361-offset entry (271 degrees), so
        // the cosine is +sin(1 deg), not 0, and the sine is ~-1.
        let angles = Angles::shared();
        assert_eq!(spark.transform.values[4], angles.cosine(-90));
        assert_eq!(spark.transform.values[8], angles.cosine(-90));
        assert_eq!(spark.transform.values[5], angles.sine(-90));
        assert_eq!(spark.transform.values[7], -angles.sine(-90));
    }

    #[test]
    fn dance2_tints_on_even_frames_from_four_through_ninety() {
        let mut effect = RisingSparkles::new(2);
        let mut runtime = runtime_with(848, 100);

        for _ in 0..5 {
            step(&mut effect, &mut runtime);
        }

        // Five steps: frames 0..4 ran; frame 4 was the first tint frame.
        assert_eq!(runtime.parent_color, Some([-1, 182, 89, 255]));
        assert_eq!(runtime.parent_flags, 1);
        assert_eq!(runtime.primitives.len(), 3, "spawns on frames 0, 2 and 4");

        step(&mut effect, &mut runtime);

        // Frame 5 is odd: the white half of the dispatch, and `begin_step`
        // cleared the flag request.
        assert_eq!(runtime.parent_color, Some([-1, 255, 255, 255]));
        assert_eq!(runtime.parent_flags, 0);
        assert_eq!(runtime.primitives.len(), 3, "odd frames spawn nothing");
    }

    #[test]
    fn dance2_does_not_tint_before_frame_four_or_after_ninety() {
        let mut effect = RisingSparkles::new(2);
        let mut runtime = runtime_with(848, 100);

        for _ in 0..4 {
            step(&mut effect, &mut runtime);
        }

        // Frames 0..3 ran; the window opens on frame 4.
        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);

        let mut effect = RisingSparkles::new(2);
        let mut runtime = runtime_with(848, 100);
        runtime.frame = 91;
        effect.on_process(&mut runtime);

        assert_eq!(runtime.parent_color, None);
        assert_eq!(runtime.parent_flags, 0);
    }

    // -- EF_THUNDERSTORM2 --------------------------------------------------

    #[test]
    fn thunderstorm2_spawns_one_persistent_depthless_bolt() {
        let mut effect = Thunderstorm2::default();
        let mut runtime = runtime_with(622, 100);

        assert!(effect.on_process(&mut runtime));

        assert_eq!(runtime.positional_sounds, vec![("effect\\EF_ThunderStorm.wav".to_owned(), [
            0.0, 0.0, 0.0
        ])]);
        assert!(runtime.sounds.is_empty());

        let bolt = &runtime.primitives[0].state;
        assert_eq!(bolt.count, PrimitiveId::Particle3d);
        assert_eq!(bolt.duration, 100);
        assert_eq!(bolt.render_flag, 0x201); // constructor RSF_ALPHA | RSF_NO_DEPTH
        assert_eq!(bolt.pattern, 0x200);
        assert_eq!(bolt.delta_pos2[1], -4.5);
        assert_eq!(bolt.anim_speed, 2);
        assert_eq!(bolt.size, 2.5);
        assert_eq!(bolt.sprite, Some("이팩트\\썬더스톰.spr"));
        assert_eq!(bolt.action, Some("이팩트\\썬더스톰.act"));
        assert_eq!(bolt.records.len(), 1);
    }

    #[test]
    fn thunderstorm2_emits_on_frame_zero_only_and_runs_100_frames() {
        let mut effect = Thunderstorm2::default();
        let mut runtime = runtime_with(622, 100);

        let snapshots = run_to_completion(&mut effect, &mut runtime);

        assert_eq!(snapshots.len(), 100);
        assert_eq!(runtime.primitives.len(), 1);
    }
}
