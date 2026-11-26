//! `CRagEffect::SonicBlowHit` - `RagEffect.cpp:2324-2364`.
//!
//! `EF_SONICBLOWHIT`: one `magic_red.tga` slash disc on frame 0, lifted by 2
//! units/frame and spread by the two random offsets.

use super::super::angles::Angles;
use super::super::math::matrix_mult;
use super::super::primitive::PrimitiveId;
use super::super::runtime::{EffectRuntime, PortedEffect};

#[derive(Debug)]
pub struct SonicBlowHit {
    lifetime: i32,
}

impl SonicBlowHit {
    pub fn new(lifetime: i32) -> Self {
        Self { lifetime }
    }
}

impl PortedEffect for SonicBlowHit {
    fn lifetime(&self) -> i32 {
        self.lifetime
    }

    fn on_process(&mut self, runtime: &mut EffectRuntime) -> bool {
        if runtime.frame != 0 {
            return true;
        }

        let angles = Angles::shared();

        runtime.play_sound("effect\\assasin_sonicblow.wav");

        let mut facing = (180.0 - runtime.parent_angle) as i32;
        let spread_x = runtime.rng.clamped_random(10);
        let spread_y = runtime.rng.clamped_random(20);

        if facing < 0 {
            facing += 360;
        }

        let disc = runtime.launch(PrimitiveId::Cylinder3d, [0.0, 0.0, 0.0]);
        let state = runtime.primitive_mut(disc);

        state.duration = 12;
        state.longitude = facing as f32;
        state.latitude = -90.0;
        state.transform.make_x_rotation(angles.radian(-90));
        state.transform.append_y_rotation(angles.radian(facing));

        let speed3d = matrix_mult([0.0, -2.0, 0.0], &state.transform.values);
        state.speed3d = speed3d;
        state.delta_pos = speed3d;
        state.delta_pos2 = [(spread_x + 1) as f32 / 4.0, (spread_y + 1) as f32 / 2.0 - 11.0, 0.0];
        state.speed = 0.4;
        state.accel = -(state.speed / state.duration as f32) * 0.5;
        state.outer_size = 7.0;
        state.inner_size = 4.0;
        state.height_size = 3.5;
        state.fade_out_cnt = state.duration - state.duration / 2;
        state.texture = Some("effect\\magic_red.tga");

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::effect::port::runtime::{EffectRuntime, step};

    #[test]
    fn sonic_blow_hit_spawns_the_red_disc() {
        let mut effect = SonicBlowHit::new(100);
        let mut runtime = EffectRuntime::new(100, [0.0, 0.0], 611);
        runtime.parent_angle = 0.0;

        step(&mut effect, &mut runtime);

        assert_eq!(runtime.sounds, vec!["effect\\assasin_sonicblow.wav".to_owned()]);
        assert_eq!(runtime.primitives.len(), 1);

        let disc = &runtime.primitives[0].state;
        assert_eq!(disc.count, PrimitiveId::Cylinder3d);
        assert_eq!(disc.duration, 12);
        assert_eq!(disc.longitude, 180.0);
        assert_eq!(disc.latitude, -90.0);
        assert_eq!(disc.outer_size, 7.0);
        assert_eq!(disc.inner_size, 4.0);
        assert_eq!(disc.height_size, 3.5);
        assert_eq!(disc.fade_out_cnt, 6);
        assert!((disc.speed - 0.4).abs() < 0.05);
        assert_eq!(disc.texture, Some("effect\\magic_red.tga"));
        assert!(disc.delta_pos2[0] >= 0.25 && disc.delta_pos2[0] <= 2.75);
        assert!(disc.delta_pos2[1] >= -10.5 && disc.delta_pos2[1] <= -0.5);
    }
}
