//! `CRagEffect::FireArrow` - `RagEffect.cpp:11533-11624`, effect id
//! `EF_FIREARROW` (31), dispatch at `RagEffect.cpp:23287-23288`.
//!
//! At frame 12 the effect plays a random wave and multiplies the hit count by
//! ten; arrow primitives (`PP_3DCROSSTEXTURE`, six `불화살N.tga` frames) then
//! leave every ten frames while `frame - 12 < count`, aimed by the *effect
//! level* `longitude`/`latitude` that `Heading` computes from the spread
//! vector. Every fourth frame spawns a `particle4.spr` mote whose direction
//! jitters around the effect angles. A `PP_3DCIRCLE` ring closes the body when
//! the skill sets `flag`.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

/// `EF_FIREARROW`'s six-frame arrow texture (`RagEffect.cpp:11572-11577`).
const FIRE_ARROW_TEXTURES: [&str; 6] = [
    "effect\\불화살1.tga",
    "effect\\불화살2.tga",
    "effect\\불화살3.tga",
    "effect\\불화살4.tga",
    "effect\\불화살5.tga",
    "effect\\불화살6.tga",
];

#[derive(Debug, Default)]
pub struct FireArrow;

impl PortedEffect for FireArrow {
    fn lifetime(&self) -> i32 {
        160
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 0 {
            // `CRagEffect::Init` (`RagEffectInitialization.cpp:22`) sets the
            // effect-level longitude from the caster; the particles on frames
            // 4/8 read it before the arrow block overwrites it.
            runtime.init_effect_angles();
        }

        if frame == 12 {
            let wave = runtime.rng.clamped_random(3) + 1;
            runtime.play_sound(&format!("effect\\EF_FireArrow{wave}.wav"));
            runtime.count *= 10;
        }

        let angles = Angles::shared();

        if frame >= 12 && (frame - 12) % 10 == 0 && frame - 12 < runtime.count {
            // RNG order: spread z, spread x, then the two heading targets.
            let spread_z = runtime.rng.clamped_random(10);
            let spread_x = runtime.rng.clamped_random(10);
            let heading_target_x = (runtime.rng.clamped_random(4) - 2) as f32;
            let heading_target_z = (runtime.rng.clamped_random(4) - 2) as f32;

            let delta_x = spread_x as f32 + 25.0;
            let delta_z = spread_z as f32 + 15.0;

            let heading = EffectRuntime::heading(delta_x, delta_z, heading_target_x, heading_target_z);
            let longitude = -heading;

            let dz = delta_z.abs();
            let dx = delta_x.abs();
            let run = (dx * dx + dz * dz).sqrt();
            let latitude = EffectRuntime::heading(-60.0, run, 0.0, 0.0);

            runtime.longitude = longitude;
            runtime.latitude = latitude;

            let index = runtime.launch(PrimitiveId::CrossTexture3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(index);

            state.duration = 70;
            state.pattern = 0x14;
            state.delta_pos2 = [delta_x, -60.0, delta_z];
            state.longitude = longitude;
            state.latitude = latitude;
            state.transform.make_x_rotation(angles.radian(latitude as i32));
            state.transform.append_y_rotation(angles.radian(longitude as i32));
            state.speed = -2.75;
            state.width_size = 14.0;
            state.height_size = 3.5;
            state.fade_out_cnt = state.duration - 20;
            state.total_texture = 6;
            state.anim_speed = 1;
            state.frame_count = 6;
            state.texture_frames = FIRE_ARROW_TEXTURES.to_vec();
            state.texture = FIRE_ARROW_TEXTURES.first().copied();
            state.slots[0].life = 0;
        }

        if frame > 0 && frame % 4 == 0 {
            // RNG order: duration, longitude, latitude, speed, size.
            let duration = runtime.rng.clamped_random(24) + 6;
            let longitude = runtime.rng.clamped_random(40) as f32 + (runtime.longitude - 20.0);
            let latitude = runtime.rng.clamped_random(20) as f32 + (runtime.latitude - 10.0);
            let speed = (runtime.rng.clamped_random(90) + 60) as f32 / 100.0;
            let size = (runtime.rng.clamped_random(30) + 20) as f32 / 100.0;
            let target_offset = runtime.target_offset;

            let index = runtime.launch(PrimitiveId::Particle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(index);

            state.duration = duration;
            state.longitude = longitude;
            state.latitude = latitude;
            state.transform.make_x_rotation(angles.radian(latitude as i32));
            state.transform.append_y_rotation(angles.radian(longitude as i32));
            state.delta_pos2 = target_offset;
            state.speed = speed;
            state.accel = -(speed / duration as f32) / 1.5;
            state.size = size;
            state.size_speed = -(size / duration as f32);
            state.fade_out_cnt = 0;
            state.sprite = Some("이팩트\\particle4.spr");
            state.action = Some("이팩트\\particle4.act");
            state.init_segment();
        }

        if runtime.flag == 0 {
            return true;
        }

        runtime.flag = 0;

        let index = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(index);

        state.duration = 30;
        state.latitude = 90.0;
        state.radius_speed = 1.2;
        let slowdown = -(state.radius_speed / (state.duration as f32 + 40.0));
        state.radius_accel = slowdown + slowdown;
        state.inner_size = 10.0;
        state.alpha = 0.0;
        state.alpha_speed = state.max_alpha / 10.0;
        state.fade_out_cnt = state.duration - 10;
        state.init_segment();
        state.texture = Some("effect\\ring_yellow.tga");

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRng, EffectRuntime};

    fn runtime(seed: u64) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(160, [0.0, 0.0], seed);
        runtime.count = 3;
        runtime.position = [5.0, 6.0, 7.0];
        runtime.target_offset = [5.0, 6.0, 7.0];
        runtime.parent_angle = 90.0;
        runtime
    }

    fn run_body_to(effect: &mut FireArrow, runtime: &mut EffectRuntime, last_frame: i32) {
        for frame in 0..=last_frame {
            runtime.frame = frame;
            effect.on_process(runtime);
        }
    }

    #[test]
    fn lifetime_is_the_official_160() {
        assert_eq!(FireArrow.lifetime(), 160);
    }

    #[test]
    fn frame_zero_initialises_the_effect_longitude_from_the_parent() {
        let mut effect = FireArrow;
        let mut runtime = runtime(1);

        runtime.frame = 0;
        effect.on_process(&mut runtime);

        assert_eq!(runtime.longitude, 90.0);
        assert_eq!(runtime.latitude, 0.0);
        assert!(runtime.primitives.is_empty());
    }

    #[test]
    fn frame_twelve_plays_a_wave_and_multiplies_the_count_by_ten() {
        let mut effect = FireArrow;
        let mut runtime = runtime(1);

        for frame in 0..12 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        let count_before = runtime.count;
        runtime.frame = 12;
        effect.on_process(&mut runtime);

        assert_eq!(count_before, 3);
        assert_eq!(runtime.count, 30);
        assert_eq!(runtime.sounds.len(), 1);
        assert!(runtime.sounds[0].starts_with("effect\\EF_FireArrow"));
        assert!(runtime.sounds[0].ends_with(".wav"));
    }

    #[test]
    fn arrow_is_aimed_by_the_effect_level_heading() {
        let seed = 7;
        let mut effect = FireArrow;
        let mut runtime = runtime(seed);

        run_body_to(&mut effect, &mut runtime, 12);

        // Replay the body's RNG consumption: frames 4 and 8 spawn one mote each
        // (24/40/20/90/30), frame 12 then draws the wave and the arrow values.
        let mut expected = EffectRng::new(seed);
        for _ in 0..2 {
            for range in [24i32, 40, 20, 90, 30] {
                let _ = expected.clamped_random(range);
            }
        }

        let _wave = expected.clamped_random(3) + 1;
        let spread_z = expected.clamped_random(10);
        let spread_x = expected.clamped_random(10);
        let target_x = (expected.clamped_random(4) - 2) as f32;
        let target_z = (expected.clamped_random(4) - 2) as f32;

        let delta_x = spread_x as f32 + 25.0;
        let delta_z = spread_z as f32 + 15.0;
        let heading = EffectRuntime::heading(delta_x, delta_z, target_x, target_z);
        let run = (delta_x * delta_x + delta_z * delta_z).sqrt();
        let latitude = EffectRuntime::heading(-60.0, run, 0.0, 0.0);
        let angles = Angles::shared();

        let arrow = runtime
            .primitives
            .iter()
            .rev()
            .map(|primitive| &primitive.state)
            .find(|state| state.count == PrimitiveId::CrossTexture3d)
            .unwrap();

        assert_eq!(arrow.duration, 70);
        assert_eq!(arrow.pattern, 0x14);
        assert_eq!(arrow.delta_pos2, [delta_x, -60.0, delta_z]);
        assert_eq!(arrow.longitude, -heading);
        assert_eq!(arrow.latitude, latitude);
        assert_eq!(runtime.longitude, -heading);
        assert_eq!(runtime.latitude, latitude);
        assert_eq!(arrow.speed, -2.75);
        assert_eq!(arrow.width_size, 14.0);
        assert_eq!(arrow.height_size, 3.5);
        assert_eq!(arrow.fade_out_cnt, 50);
        assert_eq!(arrow.total_texture, 6);
        assert_eq!(arrow.anim_speed, 1);
        assert_eq!(arrow.texture_frames, FIRE_ARROW_TEXTURES.to_vec());
        assert_eq!(arrow.texture, Some(FIRE_ARROW_TEXTURES[0]));
        assert_eq!(arrow.slots[0].life, 0);

        let mut transform = crate::world::effect::port::math::Matrix::default();
        transform.make_x_rotation(angles.radian(latitude as i32));
        transform.append_y_rotation(angles.radian(-heading as i32));
        assert_eq!(arrow.transform, transform);
    }

    #[test]
    fn one_arrow_per_ten_frames_until_the_effect_count_runs_out() {
        let mut effect = FireArrow;
        let mut runtime = runtime(11);

        run_body_to(&mut effect, &mut runtime, 32);

        let arrows = runtime
            .primitives
            .iter()
            .filter(|primitive| primitive.state.count == PrimitiveId::CrossTexture3d)
            .count();
        assert_eq!(arrows, 3, "count 3 * 10 -> frames 12, 22 and 32");
    }

    #[test]
    fn spark_motes_spawn_every_fourth_frame_after_frame_zero() {
        let mut effect = FireArrow;
        let mut runtime = runtime(3);

        run_body_to(&mut effect, &mut runtime, 12);

        let motes: Vec<_> = runtime
            .primitives
            .iter()
            .map(|primitive| &primitive.state)
            .filter(|state| state.count == PrimitiveId::Particle3d)
            .collect();

        // Frames 4, 8 and 12.
        assert_eq!(motes.len(), 3);

        for mote in &motes {
            assert!((6..=29).contains(&mote.duration));
            assert_eq!(mote.delta_pos2, [5.0, 6.0, 7.0]);
            assert!(mote.speed >= 0.6 && mote.speed < 1.5);
            assert_eq!(mote.accel, -(mote.speed / mote.duration as f32) / 1.5);
            assert!(mote.size >= 0.2 && mote.size < 0.5);
            assert_eq!(mote.size_speed, -(mote.size / mote.duration as f32));
            assert_eq!(mote.fade_out_cnt, 0);
            assert_eq!(mote.sprite, Some("이팩트\\particle4.spr"));
            assert_eq!(mote.action, Some("이팩트\\particle4.act"));
        }
    }

    #[test]
    fn flag_spawns_the_yellow_ring_and_clears_itself() {
        let mut effect = FireArrow;
        let mut runtime = runtime(5);
        runtime.flag = 1;

        runtime.frame = 0;
        effect.on_process(&mut runtime);

        assert_eq!(runtime.flag, 0);
        assert_eq!(runtime.primitives.len(), 1, "frame 0 only has the ring");

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::Circle3d);
        assert_eq!(ring.duration, 30);
        assert_eq!(ring.latitude, 90.0);
        assert_eq!(ring.radius_speed, 1.2);
        assert_eq!(ring.radius_accel, -(1.2 / 70.0) * 2.0);
        assert_eq!(ring.inner_size, 10.0);
        assert_eq!(ring.alpha, 0.0);
        assert_eq!(ring.alpha_speed, 254.0 / 10.0);
        assert_eq!(ring.fade_out_cnt, 20);
        assert_eq!(ring.texture, Some("effect\\ring_yellow.tga"));
    }
}
