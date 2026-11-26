//! `CRagEffect::BlitzBeat` - `RagEffect.cpp:11631-11710`.
//!
//! Ten spread quads on frame 0 plus up to `state.count` green cylinders from
//! frame 36 onwards (every three frames), with the two hunter waves.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct BlitzBeat {
    lifetime: i32,
    emitted: i32,
}

impl BlitzBeat {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime, emitted: 0 }
    }
}

impl PortedEffect for BlitzBeat {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        let frame = runtime.frame;
        let angles = Angles::shared();

        if frame == 5 {
            runtime.play_sound("effect\\hunter_blitzbeat_1st.wav");
        }

        if frame == 30 {
            runtime.play_sound("effect\\hunter_blitzbeat.wav");
        }

        if frame == 0 {
            let parent_angle = runtime.parent_angle;

            for _ in 0..10 {
                let spin = runtime.rng.clamped_random(360);
                let sine = angles.raw_sine(spin as usize);
                let cosine = angles.raw_cosine(spin as usize);
                let width = (runtime.rng.clamped_random(40) + 20) as f32 / 10.0;

                let quad = runtime.launch(PrimitiveId::CrossTexture3d, [0.0, 0.0, 0.0]);
                let state = runtime.primitive_mut(quad);

                state.duration = 20;
                state.render_flag |= 0x200;
                state.longitude = parent_angle;
                state.transform.make_y_rotation(angles.radian(parent_angle as i32));

                let speed3d = matrix_mult([0.0, 0.0, 15.0], &state.transform.values);
                state.speed3d = speed3d;
                state.delta_pos = speed3d;
                state.delta_pos2 = [sine * 3.5, -7.0, cosine * -3.5];
                state.speed = -1.2;
                state.accel = -0.1;
                state.width_size = width;
                state.height_size = 0.2;
                state.width_speed = 0.1;
                state.width_accel = -0.01;
                state.alpha = 0.0;
                state.max_alpha = 250.0;
                state.change_point = 10;
                state.change_speed = -0.3;
                state.change_accel = 0.001;
                state.alpha_speed = state.max_alpha * 0.5;
                state.fade_out_cnt = state.duration - 2;
                state.texture = Some("effect\\ac_center2.tga");
                state.slots[0].life = 0;
            }
        }

        if frame <= 35 || frame % 3 != 0 || self.emitted >= runtime.count {
            return true;
        }

        let facing = runtime.parent_angle as i32;
        let drift_x = runtime.rng.clamped_random(10);
        let drift_y = runtime.rng.clamped_random(20);

        let bolt = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(bolt);

        state.duration = 12;
        state.longitude = facing as f32;
        state.latitude = -90.0;
        state.transform.make_x_rotation(angles.radian(-90));
        state.transform.append_y_rotation(angles.radian(facing));

        let speed3d = matrix_mult([0.0, -2.0, 1.0], &state.transform.values);
        state.speed3d = speed3d;
        state.delta_pos = speed3d;
        state.delta_pos2 = [(drift_x + 1) as f32 / 5.0, (drift_y + 1) as f32 / 2.0 - 12.0, 0.0];
        state.speed = 0.4;
        state.accel = -(state.speed / state.duration as f32) * 0.5;
        state.outer_size = 1.5;
        state.inner_size = 0.5;
        state.height_size = 3.5;
        state.fade_out_cnt = state.duration - state.duration / 2;
        state.texture = Some("effect\\magic_green.tga");

        self.emitted += 1;

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn blitz_beat_opens_with_ten_spread_quads() {
        let mut effect = BlitzBeat::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 571);
        runtime.count = 3;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.primitives.len(), 10);

        let quad = &runtime.primitives[0].state;
        assert_eq!(quad.count, PrimitiveId::CrossTexture3d);
        assert_eq!(quad.duration, 20);
        assert_eq!(quad.render_flag & 0x200, 0x200);
        assert_eq!(quad.height_size, 0.2);
        assert_eq!(quad.max_alpha, 250.0);
        assert_eq!(quad.change_point, 10);
        assert_eq!(quad.fade_out_cnt, 18);
        assert_eq!(quad.texture, Some("effect\\ac_center2.tga"));
        assert_eq!(quad.slots[0].life, 0);
    }

    #[test]
    fn bolts_emit_after_frame_thirty_five_up_to_the_count() {
        let mut effect = BlitzBeat::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 571);
        runtime.count = 2;

        while runtime.frame <= 35 {
            step(&mut effect, &mut runtime);
        }
        assert_eq!(runtime.primitives.len(), 10, "no bolts yet");

        while runtime.frame <= 41 {
            step(&mut effect, &mut runtime);
        }

        // Frames 36 and 39 emit one bolt each.
        assert_eq!(runtime.primitives.len(), 12);
        let bolt = &runtime.primitives[10].state;
        assert_eq!(bolt.count, PrimitiveId::Cylinder3d);
        assert_eq!(bolt.duration, 12);
        assert_eq!(bolt.latitude, -90.0);
        assert_eq!(bolt.outer_size, 1.5);
        assert_eq!(bolt.inner_size, 0.5);
        assert_eq!(bolt.height_size, 3.5);
        assert_eq!(bolt.fade_out_cnt, 6);
        assert_eq!(bolt.texture, Some("effect\\magic_green.tga"));
    }

    #[test]
    fn waves_land_on_five_and_thirty() {
        let mut effect = BlitzBeat::new(250);
        let mut runtime = EffectRuntime::new(250, [0.0, 0.0], 571);
        runtime.count = 0;

        while runtime.frame < 5 {
            step(&mut effect, &mut runtime);
        }
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds, vec!["effect\\hunter_blitzbeat_1st.wav".to_owned()]);

        while runtime.frame < 30 {
            step(&mut effect, &mut runtime);
        }
        step(&mut effect, &mut runtime);
        assert_eq!(runtime.sounds, vec![
            "effect\\hunter_blitzbeat_1st.wav".to_owned(),
            "effect\\hunter_blitzbeat.wav".to_owned()
        ]);
    }
}
