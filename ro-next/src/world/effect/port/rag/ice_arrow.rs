//! `CRagEffect::IceArrow` - `RagEffect.cpp:12469-12542`, effect id
//! `EF_ICEARROW` (26), dispatch at `RagEffect.cpp:23278-23279`.
//!
//! Like [`super::FireArrow`], at frame 12 it plays a random wave and
//! multiplies the count by ten, then launches one `PP_3DCROSSTEXTURE` arrow
//! every ten frames while `frame - 12 < count`. Ice Arrow has a single
//! `icearrow.tga` frame instead of the six-frame fire sequence, and no mote
//! spray. A `PP_3DCIRCLE` `ring_blue.tga` closes the body when `flag` is set.

use super::super::angles::Angles;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug, Default)]
pub struct IceArrow;

impl PortedEffect for IceArrow {
    fn lifetime(&self) -> i32 {
        160
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;

        if frame == 0 {
            // `CRagEffect::Init` (`RagEffectInitialization.cpp:22`).
            runtime.init_effect_angles();
        }

        if frame == 12 {
            let wave = runtime.rng.clamped_random(3) + 1;
            runtime.play_sound(&format!("effect\\EF_IceArrow{wave}.wav"));
            runtime.count *= 10;
        }

        if frame >= 12 {
            let frame_offset = frame - 12;
            let interval = 10;

            if frame_offset % interval == 0 && frame_offset < runtime.count {
                // RNG order: spread z, spread x, then the two heading targets.
                let spread_z = runtime.rng.clamped_random(interval);
                let spread_x = runtime.rng.clamped_random(interval);
                let heading_target_x = (runtime.rng.clamped_random(4) - 2) as f32;
                let heading_target_z = (runtime.rng.clamped_random(4) - 2) as f32;

                let delta_x = spread_x as f32 + 25.0;
                let delta_z = spread_z as f32 + 15.0;

                let angle_y = -EffectRuntime::heading(delta_x, delta_z, heading_target_x, heading_target_z);

                let dz = delta_z.abs();
                let dx = delta_x.abs();
                let dist_xz = (dx * dx + dz * dz).sqrt();
                let angle_x = EffectRuntime::heading(-60.0, dist_xz, 0.0, 0.0);

                runtime.longitude = angle_y;
                runtime.latitude = angle_x;

                let angles = Angles::shared();

                let index = runtime.launch(PrimitiveId::CrossTexture3d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(index);

                state.duration = 70;
                state.pattern = 20;
                state.delta_pos2 = [delta_x, -60.0, delta_z];
                state.longitude = angle_y;
                state.latitude = angle_x;
                state.transform.make_x_rotation(angles.radian(angle_x as i32));
                state.transform.append_y_rotation(angles.radian(angle_y as i32));
                state.speed = -2.75;
                state.width_size = 11.5;
                state.height_size = 3.8;
                state.fade_out_cnt = state.duration - 20;
                state.texture = Some("effect\\icearrow.tga");
                state.slots[0].life = 0;
            }
        }

        if runtime.flag != 0 {
            runtime.flag = 0;

            let index = runtime.launch(PrimitiveId::Circle3d, [0.0, 0.0, 0.0]);
            let state = runtime.primitive_mut(index);

            state.duration = 30;
            state.latitude = 90.0;
            state.radius_speed = 1.2;
            let accel = -(state.radius_speed / (state.duration as f32 + 40.0));
            state.radius_accel = accel + accel;
            state.inner_size = 10.0;
            state.alpha = 0.0;
            state.alpha_speed = state.max_alpha / 10.0;
            state.fade_out_cnt = state.duration - 10;
            state.init_segment();
            state.texture = Some("effect\\ring_blue.tga");
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRng, EffectRuntime};

    fn runtime(seed: u64) -> EffectRuntime {
        let mut runtime = EffectRuntime::new(160, [0.0, 0.0], seed);
        runtime.count = 2;
        runtime.position = [5.0, 6.0, 7.0];
        runtime.target_offset = [5.0, 6.0, 7.0];
        runtime.parent_angle = 45.0;
        runtime
    }

    #[test]
    fn lifetime_is_the_official_160() {
        assert_eq!(IceArrow.lifetime(), 160);
    }

    #[test]
    fn frame_zero_initialises_the_effect_longitude() {
        let mut effect = IceArrow;
        let mut runtime = runtime(1);

        runtime.frame = 0;
        effect.on_process(&mut runtime);

        assert_eq!(runtime.longitude, 45.0);
        assert_eq!(runtime.latitude, 0.0);
    }

    #[test]
    fn frame_twelve_plays_the_ice_wave_and_scales_the_count() {
        let mut effect = IceArrow;
        let mut runtime = runtime(1);

        for frame in 0..12 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        runtime.frame = 12;
        effect.on_process(&mut runtime);

        assert_eq!(runtime.count, 20);
        assert_eq!(runtime.sounds.len(), 1);
        assert!(runtime.sounds[0].starts_with("effect\\EF_IceArrow"));
    }

    #[test]
    fn arrow_uses_the_ice_heading_and_texture() {
        let seed = 13;
        let mut effect = IceArrow;
        let mut runtime = runtime(seed);

        for frame in 0..=12 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        let mut expected = EffectRng::new(seed);
        let _wave = expected.clamped_random(3) + 1;
        let spread_z = expected.clamped_random(10);
        let spread_x = expected.clamped_random(10);
        let target_x = (expected.clamped_random(4) - 2) as f32;
        let target_z = (expected.clamped_random(4) - 2) as f32;

        let delta_x = spread_x as f32 + 25.0;
        let delta_z = spread_z as f32 + 15.0;
        let angle_y = -EffectRuntime::heading(delta_x, delta_z, target_x, target_z);
        let dist_xz = (delta_x * delta_x + delta_z * delta_z).sqrt();
        let angle_x = EffectRuntime::heading(-60.0, dist_xz, 0.0, 0.0);

        let angles = Angles::shared();

        let arrow = runtime
            .primitives
            .iter()
            .map(|primitive| &primitive.state)
            .find(|state| state.count == PrimitiveId::CrossTexture3d)
            .unwrap();

        assert_eq!(arrow.duration, 70);
        assert_eq!(arrow.pattern, 20);
        assert_eq!(arrow.delta_pos2, [delta_x, -60.0, delta_z]);
        assert_eq!(arrow.longitude, angle_y);
        assert_eq!(arrow.latitude, angle_x);
        assert_eq!(runtime.longitude, angle_y);
        assert_eq!(runtime.latitude, angle_x);
        assert_eq!(arrow.speed, -2.75);
        assert_eq!(arrow.width_size, 11.5);
        assert_eq!(arrow.height_size, 3.8);
        assert_eq!(arrow.fade_out_cnt, 50);
        assert_eq!(arrow.total_texture, 1);
        assert_eq!(arrow.texture, Some("effect\\icearrow.tga"));
        assert_eq!(arrow.slots[0].life, 0);

        let mut transform = crate::world::effect::port::math::Matrix::default();
        transform.make_x_rotation(angles.radian(angle_x as i32));
        transform.append_y_rotation(angles.radian(angle_y as i32));
        assert_eq!(arrow.transform, transform);
    }

    #[test]
    fn one_arrow_every_ten_frames_until_the_count_runs_out() {
        let mut effect = IceArrow;
        let mut runtime = runtime(9);

        for frame in 0..=32 {
            runtime.frame = frame;
            effect.on_process(&mut runtime);
        }

        let arrows = runtime
            .primitives
            .iter()
            .filter(|primitive| primitive.state.count == PrimitiveId::CrossTexture3d)
            .count();
        assert_eq!(arrows, 2, "count 2 * 10 -> frames 12 and 22");
    }

    #[test]
    fn flag_spawns_the_blue_ring() {
        let mut effect = IceArrow;
        let mut runtime = runtime(5);
        runtime.flag = 1;

        runtime.frame = 0;
        effect.on_process(&mut runtime);

        assert_eq!(runtime.flag, 0);
        assert_eq!(runtime.primitives.len(), 1);

        let ring = &runtime.primitives[0].state;
        assert_eq!(ring.count, PrimitiveId::Circle3d);
        assert_eq!(ring.duration, 30);
        assert_eq!(ring.latitude, 90.0);
        assert_eq!(ring.radius_speed, 1.2);
        assert_eq!(ring.radius_accel, -(1.2 / 70.0) * 2.0);
        assert_eq!(ring.inner_size, 10.0);
        assert_eq!(ring.alpha_speed, 254.0 / 10.0);
        assert_eq!(ring.fade_out_cnt, 20);
        assert_eq!(ring.texture, Some("effect\\ring_blue.tga"));
    }
}
